<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";

  type AccountView = { name: string; issuer: string | null };

  let accounts = $state<AccountView[]>([]);

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

  onMount(() => {
    refresh();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (pendingDelete) pendingDelete = null;
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

  <!-- BACKUP — stubbed until Phase 5. -->
  <p class="section">BACKUP</p>
  <div class="card">
    <div class="srow disabled">
      <div class="srow-text">
        <span class="srow-title">⬇ Back up accounts</span>
        <span class="srow-sub">Save the accounts file wherever you like</span>
      </div>
      <span class="soon-tag">coming next</span>
    </div>
  </div>

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
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
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

  .section {
    font-size: 10px;
    letter-spacing: 0.08em;
    color: #777b82;
    margin: 14px 2px 5px;
  }

  .card {
    background: #232529;
    border-radius: 8px;
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
  .srow.disabled {
    opacity: 0.62;
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
    background: #2a2d33;
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
  .srow-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .srow-title {
    font-size: 13px;
    color: #e6e7e9;
  }
  .srow-sub {
    font-size: 11px;
    color: #8b8f96;
  }
  .soon-tag {
    font-size: 10px;
    color: #8b8f96;
    background: #34373d;
    padding: 3px 7px;
    border-radius: 10px;
    white-space: nowrap;
  }

  /* Manage rows (Accounts) */
  .mrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 11px;
    min-height: 20px;
    border-top: 1px solid #2a2d33;
  }
  .mrow:first-child {
    border-top: none;
  }
  .empty-row {
    color: #777b82;
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
    color: #9aa0a8;
    font-size: 14px;
    cursor: pointer;
  }
  .icon:hover {
    background: #34373d;
    color: #e6e7e9;
  }
  .icon.danger:hover {
    background: #4a2426;
    color: #ff8a8f;
  }
  .rename-input {
    flex: 1;
    min-width: 0;
    background: #1b1d21;
    border: 1px solid #5b8cff;
    border-radius: 5px;
    color: #e6e7e9;
    padding: 5px 8px;
    font-size: 13px;
    outline: none;
  }
  .rename-error {
    color: #ff8a8f;
    font-size: 11px;
    margin: 2px 11px 6px;
  }

  .add {
    width: 100%;
    margin-top: 10px;
    background: #34373d;
    border: none;
    border-radius: 8px;
    color: #cfd3da;
    font-size: 13px;
    padding: 10px;
    cursor: pointer;
  }
  .add:hover {
    background: #3e424a;
  }

  /* Delete-confirm modal */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(10, 11, 13, 0.62);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 18px;
  }
  .modal {
    background: #26282d;
    border-radius: 12px;
    padding: 16px 16px 14px;
    max-width: 300px;
    text-align: center;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
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
    color: #b6bac1;
    line-height: 1.45;
    margin: 0 0 14px;
  }
  .modal-body strong {
    color: #e6e7e9;
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
    background: #34373d;
    color: #cfd3da;
  }
  .ghost:hover {
    background: #3e424a;
  }
  .danger-btn {
    background: #b3322f;
    color: #fff;
  }
  .danger-btn:hover {
    background: #c93b38;
  }
</style>
