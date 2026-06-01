<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { goto } from "$app/navigation";

  // The unlock gate (UNIFIED_PLAN §3.4): shown when the app opens encrypted+locked. A correct
  // passphrase unlocks the session (D7) and routes to the main list; a wrong one stays here.
  let password = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let inputEl = $state<HTMLInputElement | undefined>();

  const canSubmit = $derived(!busy && password.length > 0);

  async function submit() {
    if (!canSubmit) return;
    busy = true;
    error = null;
    try {
      await invoke("unlock", { password });
      goto("/");
    } catch (e) {
      error = String(e);
      password = "";
      busy = false;
      await tick();
      inputEl?.focus();
    }
  }

  onMount(() => {
    tick().then(() => inputEl?.focus());
    // Escape just hides the popover (auto-hide parity); reopening still lands on the gate.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<main>
  <div class="lock">🔒</div>
  <h1>Authr is locked</h1>
  <p class="sub">Enter your password to unlock your accounts.</p>

  <input
    bind:this={inputEl}
    bind:value={password}
    class="text"
    type="password"
    placeholder="Password"
    autocomplete="off"
    onkeydown={(e) => {
      if (e.key === "Enter") submit();
    }}
  />

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button class="primary" disabled={!canSubmit} onclick={submit}>Unlock</button>
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

  main {
    box-sizing: border-box;
    height: 100vh;
    padding: 0 22px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .lock {
    font-size: 30px;
    margin-bottom: 6px;
  }
  h1 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 4px;
  }
  .sub {
    font-size: 12px;
    color: #8b8f96;
    margin: 0 0 16px;
    line-height: 1.4;
  }

  .text {
    box-sizing: border-box;
    width: 100%;
    background: #34373d;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #e6e7e9;
    padding: 9px 10px;
    font-size: 13px;
    outline: none;
    text-align: center;
  }
  .text:focus {
    border-color: #5b8cff;
  }
  .text::placeholder {
    color: #777b82;
  }
  .error {
    font-size: 12px;
    color: #ff8a8f;
    margin: 10px 0 0;
  }

  .primary {
    width: 100%;
    margin-top: 14px;
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
</style>
