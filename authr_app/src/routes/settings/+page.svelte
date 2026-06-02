<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";

  type AccountView = { name: string; issuer: string | null };
  type ImportSummary = { added: number; skipped: number; relabeled: number };

  let accounts = $state<AccountView[]>([]);

  // Import flow (D11): pick a .authr, prompt for its password if it's encrypted, then merge.
  // `importPath` non-null ⇒ the password prompt is open for that file.
  let importPath = $state<string | null>(null);
  let importPw = $state("");
  let importPwError = $state<string | null>(null);
  let importBusy = $state(false);
  let importPwEl = $state<HTMLInputElement | undefined>();

  // One-tap result toast (success or error), auto-dismissed.
  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  // Inline-rename state: the name of the row being edited, plus the draft + any error.
  let editingName = $state<string | null>(null);
  let draftName = $state("");
  let renameError = $state<string | null>(null);
  let renameEl = $state<HTMLInputElement | undefined>();

  // Delete-confirm modal: the account pending deletion (null = closed).
  let pendingDelete = $state<string | null>(null);

  // Encryption state (UNIFIED_PLAN §3.4 E4 row): drives the Security row's On/Off display.
  let encryptionEnabled = $state(false);

  async function refresh() {
    try {
      accounts = await invoke<AccountView[]>("list_accounts");
    } catch (e) {
      console.error("list_accounts failed", e);
      accounts = [];
    }
    try {
      const s = await invoke<{ enabled: boolean; locked: boolean }>(
        "encryption_status",
      );
      encryptionEnabled = s.enabled;
    } catch (e) {
      console.error("encryption_status failed", e);
    }
  }

  async function startRename(name: string) {
    editingName = name;
    draftName = name;
    renameError = null;
    await tick();
    renameEl?.focus();
    renameEl?.select();
  }

  function cancelRename() {
    editingName = null;
    renameError = null;
  }

  async function commitRename(oldName: string) {
    const next = draftName.trim();
    if (!next || next === oldName) {
      cancelRename();
      return;
    }
    try {
      await invoke("rename_account", { name: oldName, newName: next });
      cancelRename();
      await refresh();
    } catch (e) {
      renameError = String(e);
    }
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    const name = pendingDelete;
    try {
      await invoke("delete_account", { name });
    } catch (e) {
      console.error("delete_account failed", e);
    }
    pendingDelete = null;
    await refresh();
  }

  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 4000);
  }

  // Honest, count-only toast (no secret crosses the bridge, D4).
  function summaryToast(s: ImportSummary): string {
    if (s.added === 0 && s.relabeled === 0) return "Nothing new to import";
    const parts = [`Imported ${s.added} new ${s.added === 1 ? "account" : "accounts"}`];
    if (s.relabeled) parts.push(`${s.relabeled} relabeled`);
    if (s.skipped) parts.push(`${s.skipped} already present`);
    return parts.join(" · ");
  }

  async function startImport() {
    const picked = await open({
      multiple: false,
      filters: [{ name: "Authr backup", extensions: ["authr", "json"] }],
    });
    if (typeof picked !== "string") return; // cancelled
    await attemptImport(picked, null);
  }

  // Try a merge; an encrypted file with no password opens the prompt, a wrong password
  // surfaces inline in that prompt, anything else becomes an error toast.
  async function attemptImport(path: string, password: string | null) {
    importBusy = true;
    try {
      const summary = await invoke<ImportSummary>("import_backup", {
        srcPath: path,
        password,
      });
      importPath = null;
      importPw = "";
      importPwError = null;
      showToast(summaryToast(summary));
      await refresh();
    } catch (e) {
      const msg = String(e);
      if (password === null && msg.toLowerCase().includes("encrypted")) {
        importPath = path; // open the password prompt for this file
        importPw = "";
        importPwError = null;
      } else if (importPath) {
        importPwError = msg; // already prompting → wrong password / other failure
      } else {
        showToast(msg);
      }
    } finally {
      importBusy = false;
    }
  }

  function submitImportPassword() {
    if (!importPath || !importPw) return;
    attemptImport(importPath, importPw);
  }

  function cancelImport() {
    importPath = null;
    importPw = "";
    importPwError = null;
  }

  // Focus the password field when the encrypted-import prompt opens.
  $effect(() => {
    if (importPath) tick().then(() => importPwEl?.focus());
  });

  onMount(() => {
    refresh();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (importPath) cancelImport();
        else if (pendingDelete) pendingDelete = null;
        else if (editingName) cancelRename();
        else goto("/");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<main>
  <header>
    <button class="back" onclick={() => goto("/")} title="Back">←</button>
    <h1>Settings</h1>
  </header>

  <!-- SECURITY — Encryption row (E4): set or change the password. -->
  <p class="section">SECURITY</p>
  <div class="card">
    <button class="srow srow-btn" onclick={() => goto("/settings/password")}>
      <div class="srow-text">
        <span class="srow-title">🔒 Encryption</span>
        <span class="srow-sub">
          {encryptionEnabled
            ? "Change your password"
            : "Set a password to protect this device"}
        </span>
      </div>
      <span class="state-tag" class:on={encryptionEnabled}>
        {encryptionEnabled ? "On" : "Off"}
      </span>
    </button>
  </div>

  <!-- BACKUP — export a .authr file (D6) and import/merge from one (D11). -->
  <p class="section">BACKUP</p>
  <div class="card">
    <button class="srow srow-btn" onclick={() => goto("/settings/backup")}>
      <div class="srow-text">
        <span class="srow-title">⬇ Back up accounts</span>
        <span class="srow-sub">Save an .authr file, encrypted or plain</span>
      </div>
      <span class="chev">→</span>
    </button>
    <button class="srow srow-btn" onclick={startImport} disabled={importBusy}>
      <div class="srow-text">
        <span class="srow-title">⬆ Import accounts</span>
        <span class="srow-sub">Merge accounts from an .authr file</span>
      </div>
      <span class="chev">→</span>
    </button>
  </div>
  <p class="caveat">
    Import <strong>adds</strong> accounts and never deletes — re-importing an old
    backup can bring back an account you deleted here.
  </p>

  <!-- ACCOUNTS · N — manage rows (rename / delete), then Add account. -->
  <p class="section">ACCOUNTS · {accounts.length}</p>
  <div class="card">
    {#if accounts.length === 0}
      <div class="mrow empty-row">No accounts yet</div>
    {:else}
      {#each accounts as a (a.name)}
        <div class="mrow">
          {#if editingName === a.name}
            <input
              bind:this={renameEl}
              bind:value={draftName}
              class="rename-input"
              spellcheck="false"
              autocomplete="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === "Enter") commitRename(a.name);
                else if (e.key === "Escape") cancelRename();
              }}
              onblur={() => commitRename(a.name)}
            />
          {:else}
            <span class="mname">{a.name}</span>
            <div class="actions">
              <button class="icon" title="Rename" onclick={() => startRename(a.name)}>✎</button>
              <button
                class="icon danger"
                title="Delete"
                onclick={() => (pendingDelete = a.name)}>🗑</button
              >
            </div>
          {/if}
        </div>
        {#if editingName === a.name && renameError}
          <p class="rename-error">{renameError}</p>
        {/if}
      {/each}
    {/if}
  </div>
  <button class="add" onclick={() => goto("/settings/add")}>+ Add account</button>
</main>

{#if pendingDelete}
  <!-- Delete-confirm modal — no-recovery warning ONLY; the secret is never shown (D4). -->
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) pendingDelete = null;
    }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <div class="warn">⚠</div>
      <p class="modal-title">Delete “{pendingDelete}”?</p>
      <p class="modal-body">
        Removes it from Authr. There's <strong>no recovery</strong> — you'd need the
        original secret to add it again.
      </p>
      <div class="modal-actions">
        <button class="ghost" onclick={() => (pendingDelete = null)}>Cancel</button>
        <button class="danger-btn" onclick={confirmDelete}>🗑 Delete</button>
      </div>
    </div>
  </div>
{/if}

{#if importPath}
  <!-- Encrypted-import prompt: asks for THAT file's password (independent of the live store). -->
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) cancelImport();
    }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <p class="modal-title">Encrypted backup</p>
      <p class="modal-body">
        This backup is protected. Enter <strong>its</strong> password to import.
      </p>
      <input
        bind:this={importPwEl}
        bind:value={importPw}
        class="pw-input"
        type="password"
        placeholder="Backup password"
        autocomplete="off"
        onkeydown={(e) => {
          if (e.key === "Enter") submitImportPassword();
        }}
      />
      {#if importPwError}
        <p class="pw-error">{importPwError}</p>
      {/if}
      <div class="modal-actions">
        <button class="ghost" onclick={cancelImport}>Cancel</button>
        <button
          class="primary-btn"
          disabled={!importPw || importBusy}
          onclick={submitImportPassword}>Import</button
        >
      </div>
    </div>
  </div>
{/if}

{#if toast}
  <div class="toast" role="status">{toast}</div>
{/if}

<style>
  /* Shell, header, and the section label come from app.css. */
  .card {
    background: var(--surface);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  /* Settings rows (Security / Backup) */
  .srow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 9px 11px;
    gap: 10px;
  }
  /* Divider between stacked rows in a card (e.g. the two Backup rows). */
  .srow + .srow {
    border-top: 1px solid var(--divider);
  }
  .chev {
    color: var(--text-faint);
    font-size: 13px;
    flex-shrink: 0;
  }
  .srow-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  /* Encryption row is a full-width button into /settings/password. */
  .srow-btn {
    width: 100%;
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .srow-btn:hover {
    background: var(--hover);
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
  .srow-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .srow-title {
    font-size: 13px;
    color: var(--text);
  }
  .srow-sub {
    font-size: 11px;
    color: var(--text-dim);
  }
  .caveat {
    font-size: 11px;
    color: var(--text-dim);
    line-height: 1.45;
    margin: 6px 2px 0;
  }
  .caveat strong {
    color: var(--text-soft);
  }

  /* Manage rows (Accounts) */
  .mrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 11px;
    min-height: 20px;
    border-top: 1px solid var(--divider);
  }
  .mrow:first-child {
    border-top: none;
  }
  .empty-row {
    color: var(--text-faint);
    font-size: 13px;
    justify-content: flex-start;
  }
  .mname {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 10px;
  }
  .actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .icon {
    width: 28px;
    height: 26px;
    background: transparent;
    border: none;
    border-radius: 5px;
    color: var(--text-muted);
    font-size: 14px;
    cursor: pointer;
  }
  .icon:hover {
    background: var(--control);
    color: var(--text);
  }
  .icon.danger:hover {
    background: var(--danger-soft-bg);
    color: var(--danger-text);
  }
  .rename-input {
    flex: 1;
    min-width: 0;
    background: var(--field-bg);
    border: 1px solid var(--accent);
    border-radius: 5px;
    color: var(--text);
    padding: 5px 8px;
    font-size: 13px;
    outline: none;
  }
  .rename-error {
    color: var(--danger-text);
    font-size: 11px;
    margin: 2px 11px 6px;
  }

  .add {
    width: 100%;
    margin-top: 10px;
    background: var(--control);
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-soft);
    font-size: 13px;
    padding: 10px;
    cursor: pointer;
  }
  .add:hover {
    background: var(--control-hover);
  }

  /* Delete-confirm modal */
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 18px;
  }
  .modal {
    background: var(--surface-raised);
    border-radius: var(--radius-lg);
    padding: 16px 16px 14px;
    max-width: 300px;
    text-align: center;
    box-shadow: var(--shadow-modal);
  }
  .warn {
    font-size: 22px;
  }
  .modal-title {
    font-size: 14px;
    font-weight: 600;
    margin: 4px 0 6px;
  }
  .modal-body {
    font-size: 12px;
    color: var(--text-modal);
    line-height: 1.45;
    margin: 0 0 14px;
  }
  .modal-body strong {
    color: var(--text);
  }
  .modal-actions {
    display: flex;
    gap: 8px;
  }
  .ghost,
  .danger-btn {
    flex: 1;
    border: none;
    border-radius: 7px;
    font-size: 13px;
    padding: 9px;
    cursor: pointer;
  }
  .ghost {
    background: var(--control);
    color: var(--text-soft);
  }
  .ghost:hover {
    background: var(--control-hover);
  }
  .danger-btn {
    background: var(--danger);
    color: #fff;
  }
  .danger-btn:hover {
    background: var(--danger-hover);
  }
  .primary-btn {
    flex: 1;
    border: none;
    border-radius: 7px;
    font-size: 13px;
    padding: 9px;
    cursor: pointer;
    background: var(--accent);
    color: #fff;
  }
  .primary-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .primary-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* Encrypted-import password prompt */
  .pw-input {
    box-sizing: border-box;
    width: 100%;
    background: var(--field-bg);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 9px;
    font-size: 13px;
    outline: none;
    margin: 4px 0 8px;
  }
  .pw-error {
    color: var(--danger-text);
    font-size: 11px;
    margin: 0 0 10px;
  }

  /* One-tap import result toast */
  .toast {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    max-width: 88%;
    background: var(--control);
    color: var(--text);
    font-size: 12px;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-toast);
    text-align: center;
  }
</style>
