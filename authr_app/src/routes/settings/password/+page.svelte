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

  onMount(() => {
    load();
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
  {/if}
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

  .warning {
    display: flex;
    gap: 9px;
    background: #3a2e1c;
    border: 1px solid #6b5524;
    border-radius: 8px;
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
    color: #e3cfa6;
  }
  .warning strong {
    color: #ffd479;
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
