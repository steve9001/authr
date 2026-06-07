<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";

  // Set vs. change mode is decided by the store's current state (UNIFIED_PLAN §3.4 E4).
  // `enabled` true ⇒ a password already protects the store, so we ask for the current one.
  let enabled = $state(false);
  let loading = $state(true);

  let current = $state("");
  let next = $state("");
  let confirm = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let curEl = $state<HTMLInputElement | undefined>();
  let nextEl = $state<HTMLInputElement | undefined>();

  // Remove-password flow: `removing` opens the confirm modal; the session is already unlocked
  // to reach Settings, so no password re-entry is needed.
  let removing = $state(false);
  let removeBusy = $state(false);
  let removeError = $state<string | null>(null);

  const canSubmit = $derived(
    !busy &&
      next.length > 0 &&
      confirm.length > 0 &&
      (!enabled || current.length > 0),
  );

  async function load() {
    try {
      const s = await invoke<{ enabled: boolean; locked: boolean }>(
        "encryption_status",
      );
      enabled = s.enabled;
    } catch (e) {
      console.error("encryption_status failed", e);
    }
    loading = false;
    await tick();
    (enabled ? curEl : nextEl)?.focus();
  }

  async function submit() {
    if (!canSubmit) return;
    if (next !== confirm) {
      error = "Passwords don't match";
      return;
    }
    busy = true;
    error = null;
    try {
      if (enabled) {
        await invoke("change_password", { old: current, new: next });
      } else {
        await invoke("set_password", { new: next });
      }
      goto("/settings");
    } catch (e) {
      // Surfaces a wrong current password / already-enabled error inline.
      error = String(e);
      busy = false;
    }
  }

  async function removePassword() {
    removeBusy = true;
    removeError = null;
    try {
      await invoke("disable_password");
      goto("/settings");
    } catch (e) {
      removeError = String(e);
      removeBusy = false;
    }
  }

  onMount(() => {
    load();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (removing) removing = false;
        else goto("/settings");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<main>
  <header>
    <button class="back" onclick={() => goto("/settings")} title="Back">←</button>
    <h1>{enabled ? "Change password" : "Set password"}</h1>
  </header>

  {#if !loading}
    <!-- The unrecoverable-password warning (E4): there is no reset, by design. -->
    <div class="warning">
      <span class="warn-icon">⚠</span>
      <p>
        This password encrypts Authr on this device and you'll enter it to open
        Authr. <strong>If you forget it, your accounts can't be recovered</strong> —
        there is no reset.
      </p>
    </div>

    {#if enabled}
      <label class="field-label" for="cur-pw">Current password</label>
      <input
        id="cur-pw"
        bind:this={curEl}
        bind:value={current}
        class="text"
        type="password"
        autocomplete="off"
        onkeydown={(e) => {
          if (e.key === "Enter") submit();
        }}
      />
    {/if}

    <label class="field-label" for="new-pw">New password</label>
    <input
      id="new-pw"
      bind:this={nextEl}
      bind:value={next}
      class="text"
      type="password"
      autocomplete="off"
      onkeydown={(e) => {
        if (e.key === "Enter") submit();
      }}
    />

    <label class="field-label" for="confirm-pw">Confirm password</label>
    <input
      id="confirm-pw"
      bind:value={confirm}
      class="text"
      type="password"
      autocomplete="off"
      onkeydown={(e) => {
        if (e.key === "Enter") submit();
      }}
    />

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <button class="primary" disabled={!canSubmit} onclick={submit}>
      {enabled ? "Update password" : "Set password"}
    </button>
    <button class="cancel" onclick={() => goto("/settings")}>Cancel</button>

    {#if enabled}
      <!-- Remove password: decrypt and revert to a plaintext store on this device. -->
      <div class="remove-section">
        <span class="remove-title">Remove password</span>
        <span class="remove-sub">
          Decrypt and store accounts as plain text on this device.
        </span>
        <button class="remove-btn" onclick={() => (removing = true)}>
          Remove password
        </button>
      </div>
    {/if}
  {/if}
</main>

{#if removing}
  <!-- Confirm modal — removing encryption leaves accounts unencrypted on disk. -->
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) removing = false;
    }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <div class="warn">⚠</div>
      <p class="modal-title">Remove password?</p>
      <p class="modal-body">
        Your accounts will be saved <strong>unencrypted</strong> on this device.
        Anyone with access to this computer can read them.
      </p>
      {#if removeError}
        <p class="modal-error">{removeError}</p>
      {/if}
      <div class="modal-actions">
        <button class="ghost" disabled={removeBusy} onclick={() => (removing = false)}>
          Cancel
        </button>
        <button class="danger-btn" disabled={removeBusy} onclick={removePassword}>
          Remove
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Shell, header, fields + buttons come from app.css; only the unrecoverable-password
     warning callout is unique to E4. */
  main {
    display: flex;
    flex-direction: column;
  }

  .warning {
    display: flex;
    gap: 9px;
    background: var(--warn-bg);
    border: 1px solid var(--warn-border);
    border-radius: var(--radius-md);
    padding: 10px 11px;
    margin-bottom: 4px;
  }
  .warn-icon {
    font-size: 15px;
    line-height: 1.3;
  }
  .warning p {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--warn-text);
  }
  .warning strong {
    color: var(--warn-strong);
  }

  /* Remove-password section — a divided danger zone below the change form. */
  .remove-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--divider);
  }
  .remove-title {
    font-size: 13px;
    color: var(--text);
  }
  .remove-sub {
    font-size: 11px;
    color: var(--text-dim);
    line-height: 1.45;
  }
  .remove-btn {
    align-self: flex-start;
    margin-top: 6px;
    background: var(--danger-soft-bg);
    border: none;
    border-radius: 7px;
    color: var(--danger-text);
    font-size: 13px;
    padding: 8px 14px;
    cursor: pointer;
  }
  .remove-btn:hover {
    background: var(--danger);
    color: #fff;
  }

  /* Confirm modal — mirrors the settings page's overlay/modal pattern. */
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
  .modal-error {
    color: var(--danger-text);
    font-size: 11px;
    margin: 0 0 12px;
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
  .danger-btn:hover:not(:disabled) {
    background: var(--danger-hover);
  }
  .ghost:disabled,
  .danger-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
