# Building macOS System Tray Apps in Rust

A general field guide to the patterns that make a good macOS menu-bar (system
tray) application. It assumes **Rust** as the language and is written around the
**Tauri** framework for the tray itself, but the architectural ideas are
framework-agnostic — the same patterns map cleanly onto other Rust GUI stacks
(`egui`/`eframe`, `tray-icon` + `winit`, `muda` menus, `slint`, GTK via `ksni`,
etc.). Where something is genuinely Tauri-specific it is called out, so you can
substitute the equivalent from your stack.

Nothing here is specific to any one app. Treat it as a checklist and a set of
reusable shapes rather than copy-paste code.

---

## 1. What a "tray app" actually is

A menu-bar / system-tray app has three defining traits that separate it from a
normal windowed application:

1. **It lives in the status bar, not the Dock.** There is usually no Dock icon
   and no application menu. The tray icon *is* the app's primary surface.
2. **Its windows are transient.** They appear on demand (a click, a hotkey),
   anchor near the tray icon, and disappear when they lose focus. They are not
   destroyed — they are hidden and reused.
3. **It is long-lived and mostly idle.** It launches at login or on demand and
   stays resident, doing nothing until the user summons it or a background event
   fires.

Designing well means leaning into all three: be invisible until needed, be
instant when summoned, and never accidentally quit or leave a dead Dock icon.

---

## 2. The macOS-specific setup (do this first)

These are the platform settings that turn a regular app into a proper menu-bar
agent. Get them wrong and the app will show a Dock icon, steal focus, or refuse
to record audio.

### 2.1 Hide from the Dock — `LSUIElement` / activation policy

There are two layers, and you generally want **both**:

- **Bundle level (`Info.plist`):** set `LSUIElement` to `true`. This is the
  declarative "I am an agent / accessory app" flag macOS reads at launch. It
  prevents the Dock icon and the app menu from ever appearing.
- **Runtime level:** set the activation policy to *Accessory* during startup. In
  Tauri this is `app.set_activation_policy(ActivationPolicy::Accessory)` in the
  setup hook; with raw AppKit it is `NSApp.setActivationPolicy(.accessory)`.

Setting it at runtime as well as in the plist makes the behavior resilient to
how the binary is launched (e.g. `cargo run` during development, where the plist
may not be applied). Belt and suspenders.

> **Tip:** *Accessory* (not *Prohibited*) is usually the right policy. Prohibited
> can prevent windows from receiving keyboard focus correctly; Accessory hides
> the Dock icon while still allowing normal window focus.

### 2.2 Declare every permission you touch

macOS gates hardware and sensitive resources behind usage-description strings
and entitlements. Two separate mechanisms, both required for sandboxed or
hardened-runtime builds:

- **Usage description strings** in `Info.plist` — human-readable reasons shown
  in the consent dialog. Examples: `NSMicrophoneUsageDescription`,
  `NSCameraUsageDescription`, `NSSpeechRecognitionUsageDescription`,
  `NSAppleEventsUsageDescription` (automating other apps),
  `NSCalendarsUsageDescription`, etc. **No string ⇒ the app crashes the moment
  it touches that API**, rather than showing a prompt.
- **Entitlements** in an `Entitlements.plist` referenced from the bundle config
  — capability flags for the sandbox / hardened runtime. Examples:
  `com.apple.security.device.audio-input`,
  `com.apple.security.device.camera`,
  `com.apple.security.automation.apple-events`,
  `com.apple.security.network.client`.

Rule of thumb: **declare the minimum set you actually use.** Each extra
entitlement weakens the app's security posture and complicates notarization.

### 2.3 Signing & notarization (for distribution)

For anything you ship outside your own machine, plan for Developer ID signing +
notarization early. Hardened runtime is required for notarization, and hardened
runtime is what makes the entitlements above mandatory. Doing this from day one
avoids a painful retrofit.

---

## 3. The window model: hidden, reused, anchored

The single most important pattern. Tray-app windows are **created once at
startup, kept hidden, and shown/hidden repeatedly** — never created and
destroyed per interaction.

### 3.1 Pre-declare windows, start them hidden

Declare your windows up front (in `tauri.conf.json`, or by constructing them
once at startup in a code-driven stack). Give each:

- a **stable label/identifier** you can look the window up by later
  (`"main"`, `"settings"`, …);
- `visible: false` so nothing flashes on launch;
- `skipTaskbar: true` (and no Dock presence) for the transient popover;
- sizing appropriate to a popover — small, often `resizable: false`, and for the
  main popover often `decorations: false` (no title bar / traffic lights) so it
  reads as a panel rather than a window.

Use **named constants for window labels and menu IDs**, not bare string
literals scattered through the code. A typo in `"setttings"` fails silently;
a constant fails at compile time.

### 3.2 Show on demand, anchored to the tray

When the tray icon is clicked (or a hotkey fires):

1. Look the window up by its label.
2. If it is already visible → **hide it** (toggle behavior — second click
   dismisses).
3. Otherwise → **position it near the tray icon, show it, and focus it**, in
   that order.

Positioning relative to the status-bar icon is fiddly to compute by hand.
Tauri's `tauri-plugin-positioner` provides `Position::TrayCenter` and friends and
needs the tray event forwarded to it (`positioner::on_tray_event(...)`). Other
stacks expose the icon's screen rectangle so you can place the window yourself.
Either way, **anchor to the icon** — a popover that appears in a random screen
location feels broken.

### 3.3 Auto-hide on focus loss

A popover should vanish when the user clicks away. Subscribe to the window's
focus-changed event and, when it loses focus, hide it:

```
on window "Focused(false)" → window.hide()
```

This is what makes the window feel like a native menu-bar popover rather than a
floating window. Combine it with an `Escape`-to-hide keyboard handler in the
frontend for good measure.

### 3.4 Intercept the close button — hide, don't destroy

For any **secondary, longer-lived window** (a Settings or Preferences window
that the user opens deliberately), the macOS red close button will, by default,
*destroy* the window. The next time the user opens settings, the window is gone
and your `get_window_by_label` lookup returns nothing.

The fix is to intercept the close request, **prevent the default close, and hide
instead**:

```
on window "CloseRequested" → api.prevent_close(); window.hide()
```

Now the window persists for the life of the app and can be re-shown instantly.
This is one of the most common tray-app bugs, so make it a default reflex for
every reusable window.

> **Decision guide:** transient popovers auto-hide on focus loss; deliberate
> windows hide on close-request. Both *hide*, neither *closes*. The only thing
> that truly terminates the app is the explicit Quit action (§4).

---

## 4. The tray icon and its menu

The tray icon carries two interaction channels — **direct click** and a
**context menu** — and they should do different things.

### 4.1 Build the icon

At startup, build the tray icon with:

- An **icon image** — reuse the app's default window icon, or supply a dedicated
  monochrome *template* image so macOS can tint it correctly for light/dark menu
  bars. (A flat, single-color glyph reads far better in the status bar than a
  full-color app icon.) This is a *different* icon from the bundle icon that
  shows in Finder — see §5 for how to set each.
- A **tooltip** (the app name).
- An attached **menu** (built separately, see below).
- **`show_menu_on_left_click(false)`** (or your stack's equivalent). This is key:
  left-click should trigger *your* behavior (toggle the popover), while
  right-click (or control-click) opens the context menu. If left-click also
  opened the menu, you could never use it to toggle the window.

### 4.2 Handle tray clicks deliberately

In the tray event handler, **match on the specific event you care about** rather
than reacting to every event. A robust pattern is to act only on a *left button,
button-up* click — this fires once per complete click and avoids double-firing on
press+release:

```
match event {
    Click { button: Left, state: Up, .. } => toggle_main_window(),
    _ => {}   // ignore everything else
}
```

The wildcard `_ => {}` arm is what makes this resilient: new event variants in a
future library version won't break your code.

### 4.3 Build the menu separately, key items by ID

Construct the menu in its own function and give every item a **stable string
ID** (defined as a constant). Handle selections by matching on the ID:

```
on_menu_event(id):
    SETTINGS => show_settings_window()
    QUIT     => app.exit(0)
    _        => {}    // ignore unknown ids
```

Keeping menu construction separate from event handling keeps each function small
and makes it trivial to add a new item: add the constant, build the item, add a
match arm. Always include the catch-all arm.

### 4.4 Always provide an explicit Quit

Because there is no Dock icon and (often) no app menu, **the context menu's Quit
item is frequently the only way for a user to exit.** Never omit it. It should
call the framework's clean exit (`app.exit(0)`), not `std::process::exit`, so
shutdown hooks and cleanup run.

---

## 5. App icons: the bundle icon and the tray icon are different

A tray app actually has **two** icons, set by **two** different mechanisms, that
appear in **two** different places. It's easy to set one and forget the other.

| Icon | Where it appears | How it's set | What art works |
|---|---|---|---|
| **Bundle icon** | Finder, the Applications folder, Spotlight, the ⌘-Tab switcher, the "About" panel | an `.icns` in the app bundle, referenced from `bundle.icon` in `tauri.conf.json` (`CFBundleIconName`/`CFBundleIconFile` at the raw-AppKit level) | full-color, detailed, square |
| **Tray icon** | the menu bar / status bar, and nowhere else | set in code on the tray builder (`.icon(...)`), optionally flagged as a *template* | flat, monochrome glyph |

Note the interaction with `LSUIElement = true` (§2.1): because a tray app has
**no Dock icon**, the bundle icon never shows up in the Dock — but it *still*
appears in Finder, the Applications folder, Spotlight, and ⌘-Tab. So both icons
matter; don't skip the bundle icon just because "it's only a tray app."

### 5.1 The bundle icon (Finder / Applications folder)

Generate the whole icon set from one source image rather than hand-cropping each
size:

- **Source:** a *square PNG or SVG with transparency*, ideally **1024×1024**.
- **Generate:** run `cargo tauri icon [INPUT]` (default input `./app-icon.png`;
  with a JS package manager it's `npm run tauri icon` / `pnpm tauri icon` /
  `yarn tauri icon` / `deno task tauri icon`). It writes `32x32.png`,
  `128x128.png`, `128x128@2x.png`, `icon.icns` (macOS) and `icon.ico` (Windows)
  — plus mobile assets — into `src-tauri/icons/` by default (`-o/--output` to
  change the directory).
- **Reference them:** make sure `tauri.conf.json`'s `bundle.icon` array lists the
  generated files, including the `.icns`:

```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

macOS uses the `.icns`, which packs the full Retina ladder (16/32/128/256/512 px
at both 1× and 2×). If you'd rather build it by hand, name a `.iconset` folder's
PNGs correctly and run `iconutil -c icns MyIcon.iconset`. In a non-Tauri AppKit
app the bundle icon is simply the `.icns` named by `CFBundleIconName` /
`CFBundleIconFile` in `Info.plist`.

> **Cache gotcha:** macOS aggressively caches bundle icons via Launch Services.
> If a rebuilt app still shows the *old* icon in Finder, that's the icon cache,
> not your build — re-copy the app into `/Applications`, or relaunch Finder/Dock,
> to see the new one.

### 5.2 The tray icon (menu bar) — use a *template* image

The status-bar icon is set in code, independently of the bundle icon, and wants
*different* art (this is the "how" behind the template-image tip in §4.1):

- Provide a **flat, single-color glyph**, not the full-color app icon — fine
  detail and color are lost at menu-bar size.
- Flag it as a **template image** so macOS tints it automatically for light vs.
  dark menu bars and for the highlighted (clicked) state:
  - Tauri (Rust): `TrayIconBuilder::new().icon_as_template(true).icon(img)`;
  - Tauri (config): set `app.trayIcon.iconAsTemplate = true`;
  - Tauri (JS): `tray.setIconAsTemplate(true)`;
  - raw AppKit: `nsImage.isTemplate = true`.
- A template image is defined **entirely by its alpha channel** — macOS ignores
  the RGB color and uses only per-pixel opacity. So author it as a **solid black
  shape on transparency**, using partial opacity for shading. (Naming the asset
  `…Template.pdf` / `…Template.png` makes AppKit treat it as a template image
  automatically.)
- Tauri's tray `iconPath` must point to a **PNG** on macOS/Linux. A single SVG/PDF
  or a 1×/2× PNG pair are the usual menu-bar asset forms.

> **Tauri v2 gotcha:** updating the tray icon **resets** the template flag, and
> calling `set_icon_as_template` afterward causes a visible blink. When changing
> the icon at runtime, set the image and template flag together
> (`set_icon_with_as_template`) instead.

---

## 6. The frontend ⇄ backend boundary (for web-UI stacks)

If your GUI is HTML/JS (Tauri, Wry, Electron-style), keep a clean, narrow bridge
between the UI and the Rust core. If your GUI is pure-Rust (egui, slint), this
section collapses into ordinary function calls — but the *separation of
concerns* still applies.

### 6.1 Commands are the API surface

Expose backend functionality as a small set of explicitly-registered **command
functions** (`#[tauri::command]` + `invoke_handler![...]`). Each command:

- takes plain, serializable arguments and returns a serializable result;
- returns `Result<T, String>` (or a richer error type) so the frontend can
  distinguish success from failure and show a message;
- does **one** clear thing (`get_settings`, `update_settings`, `get_theme`,
  `process_text`, …).

Think of the command list as the app's internal API. Keep it minimal and
stable; the frontend should never reach into Rust internals any other way.

### 6.2 Do blocking and long work off the UI path

Any command that shells out, hits the network, touches a model, or otherwise
blocks must not stall the UI:

- Mark it `async` and run the blocking part on a worker thread
  (`tauri::async_runtime::spawn_blocking`, or `tokio::task::spawn_blocking`).
- On the frontend, show a spinner / disable the trigger button while it runs,
  and re-enable on completion — success *or* error.

A popover that freezes while "thinking" feels broken; an async command plus a
visible busy state feels responsive.

### 6.3 React to window lifecycle in the UI

The frontend can listen for window events too (e.g. a `focus` event when the
popover opens) and use them to **refocus the primary input, reset transient
state, or reload preferences**. Small touches like auto-focusing the text field
when the popover appears are what make a tray app feel instant.

### 6.4 Lock down the capability / permission surface

Tauri's capability files (and the equivalent CSP / permission config in other
web-UI stacks) should grant each window **only** the permissions it needs:

- Scope capabilities **per window** — the settings window rarely needs the same
  powers as the main one.
- Only enable the specific permissions you use (window hide, shell execute,
  clipboard, …). Don't grant a blanket allow-list.
- Set a restrictive **Content Security Policy** so the webview can't load or
  connect to anything unexpected.

This is defense-in-depth: even if the UI is compromised, it can't reach beyond
its declared scope.

---

## 7. Persistence: settings, themes, and state

Tray apps almost always need to remember a little state across launches.

### 7.1 Use the OS config directory

Write to the platform's per-app config directory (`app_config_dir()` in Tauri;
the `dirs`/`directories` crates otherwise → `~/Library/Application Support/<id>/`
on macOS). **Create the directory if missing** before writing. Never write next
to the executable — the app bundle is read-only once installed.

### 7.2 Serialize to JSON, degrade gracefully

A robust load path **never panics on bad input**:

1. If the file doesn't exist → return defaults.
2. If it exists but is unreadable or malformed → return defaults (don't crash).
3. Otherwise parse it.

In Rust this is the `unwrap_or_default()` / `unwrap_or_else(...)` chain over
`fs::read_to_string` + `serde_json::from_str`. A `#[derive(Default)]` (or a
hand-written `Default`) on your settings struct gives you the fallback for free.
The guiding principle: **a corrupt config file should reset to defaults, not
brick the app.**

### 7.3 Validate on write

When saving user-edited settings, validate before persisting and return a clear
error the UI can display (e.g. "template must contain `{placeholder}`"). Catching
bad input at save time is far better than failing mysteriously later.

### 7.4 Keep separate concerns in separate files (optional)

It's fine to split unrelated state into separate small files (e.g. one for
functional settings, one for the active theme). Smaller files mean a corrupt
theme file can't take down functional settings, and each load path stays simple.

### 7.5 Theming (if you offer it)

A clean theming pattern for web UIs: ship one CSS file per theme, store only the
**active theme name** in config, and on load swap the stylesheet `href` and set a
`data-theme` attribute on the root element. Apply the theme **immediately** in
the UI for responsiveness, then persist the choice to the backend. Re-apply on
window focus so a theme changed in one window reflects in another.

---

## 8. Running external work safely

Many tray utilities exist to *trigger* something — run a CLI, call a script,
invoke a model. If you shell out, do it carefully.

- **Escape all interpolated values.** Anything from the user or from untrusted
  text that lands in a shell command must be shell-escaped (`shell-escape`
  crate) or, better, passed as discrete `Command` args rather than concatenated
  into a `sh -c` string. Naive string interpolation is a command-injection hole.
- **If you must template a command,** keep the user-controlled value confined to
  a single, clearly-marked placeholder and escape it on substitution.
- **When mixing instructions with untrusted text** (e.g. feeding user content to
  an LLM), wrap the untrusted span in loud, unambiguous delimiters so the
  instruction layer can't be hijacked by the content. This is the prompt-level
  analogue of escaping.
- **Run it on a blocking thread** (§6.2) and surface a non-zero exit code's
  `stderr` back to the user as an error — don't swallow failures.
- **Give it a scratch working directory.** A per-process temp dir
  (`std::env::temp_dir().join(format!("<app>-{}", process::id()))`) keeps any
  files the subprocess writes isolated and easy to clean up.

---

## 9. Application structure & lifecycle

A clean tray-app `setup`/startup routine reads as a short, ordered checklist:

```
run():
    builder
        .plugins(...)               // positioner, shell, global-hotkey, etc.
        .commands(...)              // the registered command list
        .setup(|app| {
            set_activation_policy(Accessory)   // hide from Dock at runtime
            prepare_runtime_state(app)         // temp dirs, managed state
            menu = build_tray_menu(app)        // construct menu (own fn)
            setup_tray_icon(app, menu)         // icon + click/menu handlers (own fn)
            setup_window_autohide(app)         // focus-loss + close-intercept (own fn)
            Ok(())
        })
        .run(context)
```

Principles visible in that shape:

- **One responsibility per function.** Menu construction, tray wiring, and
  window behavior each live in their own function. The setup hook just sequences
  them. This keeps any single piece easy to read and change.
- **Share state through the framework's managed state**, not globals. Register
  shared values (temp dir, config handle, runtime caches) with `app.manage(...)`
  and pull them out in commands via `app.state::<T>()`. This stays thread-safe
  and testable.
- **Fail loudly at startup, gracefully at runtime.** It's fine for `setup` to
  `?`-propagate a fatal error (no icon, no temp dir → the app genuinely can't
  run). But per-interaction paths (loading config, handling a click) should
  degrade rather than crash.
- **`main` stays tiny.** Keep the real logic in a library crate (`_lib`) and let
  `main.rs` just call into it. This also enables the
  `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` guard and
  mobile entry points without cluttering the core.

---

## 10. Cross-stack portability cheat-sheet

If you swap Tauri for another Rust GUI, here's where each concept lives:

| Concept | Tauri | Lower-level / alternative |
|---|---|---|
| Tray icon + menu | `TrayIconBuilder`, `MenuBuilder` | `tray-icon` + `muda` crates |
| Tray icon image / template | `.icon(...)` + `.icon_as_template(true)` | `tray-icon` `Icon` + `NSImage.isTemplate` via `objc2` |
| Bundle (Finder) icon | `bundle.icon` array + `cargo tauri icon` | `.icns` via `iconutil` + `CFBundleIconName` in `Info.plist` |
| Hide from Dock | `set_activation_policy(Accessory)` + `LSUIElement` | `NSApp.setActivationPolicy` via `objc2`/`cocoa` |
| Window by label | `get_webview_window("label")` | you hold the `Window`/`WindowId` yourself |
| Positioning near tray | `tauri-plugin-positioner` | tray icon rect → place window via `winit` |
| Focus-loss / close events | `on_window_event(WindowEvent::…)` | `winit` `WindowEvent::Focused` / `CloseRequested` |
| UI ⇄ logic bridge | `#[command]` + `invoke` | direct Rust calls (egui/slint) |
| Config dir | `app.path().app_config_dir()` | `directories` / `dirs` crate |
| Background work | `tauri::async_runtime::spawn_blocking` | `tokio` / `std::thread` |
| Global hotkey | `tauri-plugin-global-shortcut` | `global-hotkey` crate |

The *behaviors* — hide from Dock, anchor to the icon, toggle on click, auto-hide
on blur, intercept close, persist to the config dir, run blocking work off the
UI thread — are constant. Only the API names change.

---

## 11. Quick checklist for a new tray app

- [ ] `LSUIElement` in `Info.plist` **and** Accessory activation policy at runtime
- [ ] Usage-description strings + entitlements for every gated resource you touch
- [ ] Windows pre-declared, `visible: false`, looked up by stable label constants
- [ ] Tray icon with a template/monochrome image (`icon_as_template`/`iconAsTemplate`), tooltip, and attached menu
- [ ] Bundle `.icns` generated (`cargo tauri icon`) and listed in `bundle.icon` — the Finder/Applications icon, distinct from the tray icon
- [ ] Left-click toggles the popover (menu suppressed on left-click)
- [ ] Popover anchored to the tray icon and focused on show
- [ ] Auto-hide the popover on focus loss; `Escape` hides too
- [ ] Secondary windows intercept close-request and hide instead of destroying
- [ ] Explicit **Quit** menu item calling a clean framework exit
- [ ] Narrow, explicitly-registered command/API surface; blocking work off-thread
- [ ] Config in the OS config dir, JSON, defaults on missing/corrupt, validate on save
- [ ] Per-window least-privilege capabilities + restrictive CSP
- [ ] Shell/subprocess inputs escaped or passed as discrete args
- [ ] `main` thin; logic in a lib crate; setup wired as small single-purpose functions

---

*This guide describes patterns, not a particular implementation. Adapt the
specifics to your framework and app; keep the behaviors.*
