<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import { downloadDir, homeDir, join } from "@tauri-apps/api/path";
  import { goto } from "$app/navigation";

  // E6 (UNIFIED_PLAN §5, D6): the backup gets its OWN password, independent of the live
  // store. A non-empty password ⇒ the .authr is encrypted with it; left blank ⇒ plaintext
  // JSON, gated behind an explicit confirmation.
  const FILE_NAME = "authr-vault.authr";

  let password = $state("");
  let confirm = $state("");
  let plaintextConfirmed = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let pwEl = $state<HTMLInputElement | undefined>();

  // Encrypted vs. plaintext is decided purely by whether a password was typed.
  const encrypted = $derived(password.length > 0);
  const canSave = $derived(
    !busy &&
      (encrypted
        ? confirm.length > 0
        : plaintextConfirmed), // plaintext path needs the explicit opt-in
  );

  async function exportNow() {
    if (!canSave) return;
    if (encrypted && password !== confirm) {
      error = "Passwords don't match";
      return;
    }
    // Anchor the picker at Downloads (the conventional "a file I just exported" spot, one
    // click from Finder's sidebar) with the filename prefilled, so the user clearly sees
    // where it starts. Fall back to the home dir if Downloads can't be resolved.
    let defaultPath: string;
    try {
      let baseDir: string;
      try {
        baseDir = await downloadDir();
      } catch {
        baseDir = await homeDir();
      }
      defaultPath = await join(baseDir, FILE_NAME);
    } catch (e) {
      error = String(e);
      return;
    }

    // Pick a destination via the dialog plugin (user-selected path only). Suspend the
    // popover's focus-loss auto-hide while the native sheet is in front, then resume it —
    // otherwise the popover hides on focus loss and tears the sheet down with it.
    let dest: string | null;
    await invoke("set_dialog_open", { open: true });
    try {
      dest = await save({
        defaultPath,
        filters: [{ name: "Authr backup", extensions: ["authr"] }],
      });
    } catch (e) {
      error = String(e);
      return;
    } finally {
      await invoke("set_dialog_open", { open: false });
    }
    if (!dest) return; // cancelled the save dialog

    busy = true;
    error = null;
    try {
      // `Some(pw)` encrypts (D6); `null` writes plaintext JSON after the confirmation above.
      await invoke("export_backup", {
        destPath: dest,
        password: encrypted ? password : null,
      });
      goto("/settings");
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  onMount(() => {
    pwEl?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") goto("/settings");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<main>
  <header>
    <button class="back" onclick={() => goto("/settings")} title="Back">←</button>
    <h1>Back up accounts</h1>
  </header>

  <!-- The file the export produces. -->
  <div class="file-card">
    <span class="file-icon">🗄</span>
    <div class="file-text">
      <span class="file-name">{FILE_NAME}</span>
      <span class="file-sub">
        {encrypted ? "Encrypted with the password below" : "Plain text — readable by anyone"}
      </span>
    </div>
    <span class="state-tag" class:on={encrypted}>{encrypted ? "Encrypted" : "Plain"}</span>
  </div>

  <p class="explain">
    Set a password to encrypt this backup. It's <strong>separate</strong> from your
    device password — pick anything you like for the copy. Leave it blank to export
    readable JSON instead.
  </p>

  <label class="field-label" for="bk-pw">Backup password</label>
  <input
    id="bk-pw"
    bind:this={pwEl}
    bind:value={password}
    class="text"
    type="password"
    placeholder="Leave blank for plain text"
    autocomplete="off"
    onkeydown={(e) => {
      if (e.key === "Enter") exportNow();
    }}
  />

  {#if encrypted}
    <label class="field-label" for="bk-confirm">Confirm password</label>
    <input
      id="bk-confirm"
      bind:value={confirm}
      class="text"
      type="password"
      autocomplete="off"
      onkeydown={(e) => {
        if (e.key === "Enter") exportNow();
      }}
    />
  {:else}
    <!-- Plaintext path requires an explicit, deliberate opt-in (UNIFIED_PLAN §5 / D6). -->
    <label class="plain-confirm">
      <input type="checkbox" bind:checked={plaintextConfirmed} />
      <span>I understand this file stores my secrets in <strong>plain text</strong>.</span>
    </label>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button class="primary" disabled={!canSave} onclick={exportNow}>
    {encrypted ? "Save encrypted backup" : "Save plain-text backup"}
  </button>
  <button class="cancel" onclick={() => goto("/settings")}>Cancel</button>
</main>

<style>
  /* Shell, header, fields + buttons come from app.css; the file card, the encrypted/plain
     state chip, the explanation, and the plaintext opt-in are unique to E6. */
  main {
    display: flex;
    flex-direction: column;
  }

  .file-card {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--surface);
    border-radius: var(--radius-md);
    padding: 11px;
  }
  .file-icon {
    font-size: 20px;
  }
  .file-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .file-name {
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--text);
  }
  .file-sub {
    font-size: 11px;
    color: var(--text-dim);
  }
  .state-tag {
    font-size: 10px;
    color: var(--text-dim);
    background: var(--control);
    padding: 3px 8px;
    border-radius: 10px;
    white-space: nowrap;
  }
  .state-tag.on {
    color: var(--ok);
    background: var(--ok-bg);
  }

  .explain {
    font-size: 12px;
    color: var(--text-dim);
    line-height: 1.45;
    margin: 12px 2px 4px;
  }
  .explain strong {
    color: var(--text-soft);
  }

  .plain-confirm {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 12px 2px 2px;
    font-size: 12px;
    color: var(--text-modal);
    line-height: 1.4;
    cursor: pointer;
  }
  .plain-confirm input {
    margin-top: 1px;
  }
  .plain-confirm strong {
    color: var(--warn-strong);
  }
</style>
