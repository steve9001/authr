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
</style>
