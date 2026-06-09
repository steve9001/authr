<script lang="ts">
  import { onMount, tick } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import Modal from "$lib/Modal.svelte";
  import { onEscape } from "$lib/keys";
  import { backupBaseDir, withDialogGuard } from "$lib/dialog";
  import {
    listAccounts,
    encryptionStatus,
    renameAccount,
    deleteAccount,
    importBackup,
    type AccountView,
    type ImportSummary,
  } from "$lib/backend";

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
      accounts = await listAccounts();
    } catch (e) {
      console.error("list_accounts failed", e);
      accounts = [];
    }
    try {
      const s = await encryptionStatus();
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
      await renameAccount(oldName, next);
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
      await deleteAccount(name);
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
    // Anchor the picker at Downloads (where an exported backup is most likely to be),
    // falling back to the home dir. No filename — it's a file picker, not a save.
    const defaultPath = await backupBaseDir();

    // The guard suspends the popover's focus-loss auto-hide while the native open sheet is
    // in front, then resumes it (otherwise the popover hides and tears the sheet down).
    const picked = await withDialogGuard(() =>
      open({
        multiple: false,
        defaultPath,
        filters: [{ name: "Authr backup", extensions: ["authr", "json"] }],
      }),
    );
    if (typeof picked !== "string") return; // cancelled
    await attemptImport(picked, null);
  }

  // Try a merge; an encrypted file with no password opens the prompt, a wrong password
  // surfaces inline in that prompt, anything else becomes an error toast.
  async function attemptImport(path: string, password: string | null) {
    importBusy = true;
    try {
      const summary = await importBackup(path, password);
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
    return onEscape(() => {
      if (importPath) cancelImport();
      else if (pendingDelete) pendingDelete = null;
      else if (editingName) cancelRename();
      else goto("/");
    });
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
  <Modal onclose={() => (pendingDelete = null)}>
    <div class="warn">⚠</div>
    <p class="modal-title">Delete “{pendingDelete}”?</p>
    <p class="modal-body">
      Removes it from Authr. There's <strong>no recovery</strong> — you'd need the
      original secret to add it again.
    </p>
    {#snippet actions()}
      <button class="ghost" onclick={() => (pendingDelete = null)}>Cancel</button>
      <button class="danger-btn" onclick={confirmDelete}>🗑 Delete</button>
    {/snippet}
  </Modal>
{/if}

{#if importPath}
  <!-- Encrypted-import prompt: asks for THAT file's password (independent of the live store). -->
  <Modal onclose={cancelImport}>
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
    {#snippet actions()}
      <button class="ghost" onclick={cancelImport}>Cancel</button>
      <button
        class="primary-btn"
        disabled={!importPw || importBusy}
        onclick={submitImportPassword}>Import</button
      >
    {/snippet}
  </Modal>
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
  /* The On/Off state chip is a shared primitive in app.css (.state-tag / .state-tag.on). */
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

  /* Encrypted-import password prompt — the modal shell/buttons live in app.css; only the
     password field + its inline error are unique to this prompt. */
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
