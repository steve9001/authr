<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
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
    // Pick a destination via the dialog plugin (user-selected path only).
    let dest: string | null;
    try {
      dest = await save({
        defaultPath: FILE_NAME,
        filters: [{ name: "Authr backup", extensions: ["authr"] }],
      });
    } catch (e) {
      error = String(e);
      return;
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
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: #1b1d21;
    color: #e6e7e9;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  :global(body) {
    overflow-y: auto;
  }

  main {
    box-sizing: border-box;
    padding: 8px 10px 16px;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }
  .back {
    width: 28px;
    height: 28px;
    background: #34373d;
    border: none;
    border-radius: 6px;
    color: #c7c9cd;
    font-size: 15px;
    cursor: pointer;
  }
  .back:hover {
    background: #3e424a;
  }
  h1 {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }

  .file-card {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #232529;
    border-radius: 8px;
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
    font-family: "SF Mono", ui-monospace, "Menlo", monospace;
    color: #e6e7e9;
  }
  .file-sub {
    font-size: 11px;
    color: #8b8f96;
  }
  .state-tag {
    font-size: 10px;
    color: #8b8f96;
    background: #34373d;
    padding: 3px 8px;
    border-radius: 10px;
    white-space: nowrap;
  }
  .state-tag.on {
    color: #4ec98a;
    background: #1f3a2c;
  }

  .explain {
    font-size: 12px;
    color: #8b8f96;
    line-height: 1.45;
    margin: 12px 2px 4px;
  }
  .explain strong {
    color: #cfd3da;
  }

  .field-label {
    font-size: 11px;
    color: #8b8f96;
    margin: 10px 2px 4px;
  }
  .text {
    box-sizing: border-box;
    width: 100%;
    background: #34373d;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #e6e7e9;
    padding: 8px 9px;
    font-size: 13px;
    outline: none;
  }
  .text:focus {
    border-color: #5b8cff;
  }
  .text::placeholder {
    color: #777b82;
  }

  .plain-confirm {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 12px 2px 2px;
    font-size: 12px;
    color: #b6bac1;
    line-height: 1.4;
    cursor: pointer;
  }
  .plain-confirm input {
    margin-top: 1px;
  }
  .plain-confirm strong {
    color: #ffd479;
  }

  .error {
    font-size: 12px;
    color: #ff8a8f;
    margin: 10px 2px 0;
  }

  .primary {
    margin-top: 16px;
    background: #5b8cff;
    border: none;
    border-radius: 8px;
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    padding: 11px;
    cursor: pointer;
  }
  .primary:hover:not(:disabled) {
    background: #6f9bff;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .cancel {
    margin-top: 8px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: #9aa0a8;
    font-size: 13px;
    padding: 9px;
    cursor: pointer;
  }
  .cancel:hover {
    background: #2a2d33;
    color: #e6e7e9;
  }
</style>
