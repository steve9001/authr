<script lang="ts">
  import { onMount, tick } from "svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { goto } from "$app/navigation";
  import { onEscape } from "$lib/keys";
  import {
    encryptionStatus,
    getCodes,
    resizeMain,
    type CodeView,
  } from "$lib/backend";

  let codes = $state<CodeView[]>([]);
  let filter = $state("");
  let nowMs = $state(Date.now());
  let copiedName = $state<string | null>(null);
  let locked = $state(false);
  let searchEl: HTMLInputElement | undefined;
  let mainEl: HTMLElement | undefined;
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  // The unlock gate (UNIFIED_PLAN §3.4): if the store is encrypted+locked, divert to /unlock
  // before touching codes. Returns true when locked (and navigation was kicked off).
  async function gateIfLocked(): Promise<boolean> {
    try {
      const s = await encryptionStatus();
      if (s.enabled && s.locked) {
        locked = true;
        goto("/unlock");
        return true;
      }
    } catch (e) {
      console.error("encryption_status failed", e);
    }
    return false;
  }

  async function refresh() {
    if (locked) return;
    try {
      codes = await getCodes();
    } catch (e) {
      console.error("get_codes failed", e);
      codes = [];
    }
  }

  // Content-size the popover (UNIFIED tray-appearance §2): measure the natural content height
  // and ask Rust to resize + re-anchor under the tray. Measure rather than compute so it stays
  // correct as the name font / row height change. `main` is pinned to 100vh, so its scrollHeight
  // reads the viewport, not the content — instead sum the fixed chrome (everything above the
  // list) with the list's *unclipped* scrollHeight, which is robust at any current window size.
  async function fitWindow() {
    await tick(); // let the DOM reflect the current codes
    if (!mainEl) return;
    const listEl = mainEl.querySelector<HTMLElement>(".list");
    const chrome = mainEl.scrollHeight - (listEl?.clientHeight ?? 0);
    const desired = Math.ceil(chrome + (listEl?.scrollHeight ?? 0));
    try {
      await resizeMain(desired);
    } catch (e) {
      console.error("resize_main failed", e);
    }
  }

  // Reflow the window when the account count changes (add / import / delete). Keyed on the full
  // list length, NOT the filtered view — typing in search must not resize on every keystroke.
  $effect(() => {
    codes.length; // track
    fitWindow();
  });

  // Substring filter on name, case-insensitive — immediate, no debounce.
  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return codes;
    return codes.filter((c) => c.name.toLowerCase().includes(q));
  });

  // Single global countdown — every code shares the same 30s boundary, so the first
  // code's validity drives the whole bar (UNIFIED_PLAN E1: one global timer).
  const periodSeconds = $derived(codes[0]?.period_seconds ?? 30);
  const validUntil = $derived(codes[0]?.valid_until_unix ?? 0);
  const remaining = $derived(Math.max(0, validUntil - nowMs / 1000));
  const fraction = $derived(
    periodSeconds > 0 ? Math.min(1, remaining / periodSeconds) : 0,
  );
  const secondsLeft = $derived(Math.ceil(remaining));

  function grouped(code: string): string {
    return code.length === 6 ? `${code.slice(0, 3)} ${code.slice(3)}` : code;
  }

  async function copy(c: CodeView) {
    try {
      await writeText(c.code);
      copiedName = c.name;
      clearTimeout(copyTimer);
      // Long enough to read the "✓ copied!" confirmation, short enough to feel snappy.
      copyTimer = setTimeout(() => (copiedName = null), 1200);
    } catch (e) {
      console.error("clipboard write failed", e);
    }
  }

  onMount(() => {
    // Gate on the lock state first; only load codes if the store is open.
    gateIfLocked().then(async (diverted) => {
      if (!diverted) {
        await refresh();
        searchEl?.focus();
        fitWindow();
      }
    });

    // Drive the bar and re-fetch on the real period rollover (not a client-side guess).
    const tick = setInterval(() => {
      nowMs = Date.now();
      if (validUntil && nowMs / 1000 >= validUntil) {
        refresh();
      }
    }, 250);

    // Refresh + refocus the filter each time the popover reopens — re-checking the lock
    // gate too, in case the session ended while hidden.
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        gateIfLocked().then(async (diverted) => {
          if (!diverted) {
            await refresh();
            searchEl?.focus();
            fitWindow();
          }
        });
      }
    });

    const offEscape = onEscape(() => win.hide());

    return () => {
      clearInterval(tick);
      unlisten.then((u) => u());
      offEscape();
      clearTimeout(copyTimer);
    };
  });
</script>

<main bind:this={mainEl}>
  <!-- Countdown bar — single global timer for when all codes roll. Hidden when there are no
       accounts (a countdown to nothing reads oddly); the empty state below carries the screen. -->
  {#if codes.length > 0}
    <div class="countdown">
      <div class="track">
        <div class="fill" style="width: {fraction * 100}%"></div>
      </div>
      <span class="secs">{secondsLeft}s</span>
    </div>
  {/if}

  <!-- Search + gear bar. -->
  <div class="searchbar">
    <input
      bind:this={searchEl}
      bind:value={filter}
      class="search"
      type="text"
      placeholder="Search…"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
    />
    <button class="gear" title="Settings" onclick={() => goto("/settings")}>
      ⚙
    </button>
  </div>

  <!-- Account list — whole row is the tap-to-copy target. -->
  <div class="list">
    {#if filtered.length === 0}
      <p class="empty">
        {codes.length === 0 ? "No accounts yet" : "No matches"}
      </p>
    {:else}
      {#each filtered as c (c.name)}
        <button class="row" onclick={() => copy(c)}>
          <span class="name">{c.name}</span>
          {#if copiedName === c.name}
            <span class="copied">✓ copied!</span>
          {:else}
            <span class="code">{grouped(c.code)}</span>
          {/if}
        </button>
      {/each}
    {/if}
  </div>
</main>

<style>
  /* The popover is a fixed-height panel: <main> fills the viewport and the list scrolls
     inside it, so the countdown + search bar stay pinned and the body never overflows. */
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: var(--pad-y) var(--pad-x);
    gap: 8px;
  }

  /* Countdown bar */
  .countdown {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .track {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--control);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.25s linear;
  }
  .secs {
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    color: var(--text-dim);
    min-width: 26px;
    text-align: right;
  }

  /* Search + gear */
  .searchbar {
    display: flex;
    gap: 6px;
  }
  .search {
    flex: 1;
    min-width: 0;
    background: var(--control);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 6px 9px;
    font-size: 13px;
    outline: none;
  }
  .search:focus {
    border-color: var(--accent);
  }
  .search::placeholder {
    color: var(--text-faint);
  }
  .gear {
    width: 32px;
    background: var(--control);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-oncontrol);
    font-size: 15px;
    cursor: pointer;
  }
  .gear:hover {
    background: var(--control-hover);
  }

  /* Account list */
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    padding: 9px 10px;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  .row:active {
    background: var(--control);
  }
  .name {
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 10px;
  }
  .code {
    font-family: var(--font-mono);
    font-size: 15px;
    letter-spacing: 0.06em;
    color: var(--text-strong);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .copied {
    font-size: 13px;
    color: var(--ok);
    white-space: nowrap;
  }
  .empty {
    color: var(--text-faint);
    font-size: 13px;
    text-align: center;
    margin-top: 24px;
  }
</style>
