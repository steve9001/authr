<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";

  let name = $state("");
  let secret = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let nameEl: HTMLInputElement | undefined;

  const canSubmit = $derived(name.trim().length > 0 && secret.trim().length > 0 && !busy);

  async function submit() {
    if (!canSubmit) return;
    busy = true;
    error = null;
    try {
      await invoke("add_account", { name: name.trim(), secret });
      goto("/settings");
    } catch (e) {
      // Surfaces duplicate-name / invalid-secret errors from core inline.
      error = String(e);
      busy = false;
    }
  }

  onMount(() => {
    tick().then(() => nameEl?.focus());
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
    <h1>Add account</h1>
  </header>

  <label class="field-label" for="acct-name">Account name</label>
  <input
    id="acct-name"
    bind:this={nameEl}
    bind:value={name}
    class="text"
    placeholder="GitHub"
    spellcheck="false"
    autocomplete="off"
    autocapitalize="off"
    onkeydown={(e) => {
      if (e.key === "Enter") submit();
    }}
  />

  <label class="field-label" for="acct-secret">Secret key</label>
  <textarea
    id="acct-secret"
    bind:value={secret}
    class="secret"
    rows="3"
    placeholder="JBSWY3DPEHPK 3PXP DEFG H2QR"
    spellcheck="false"
    autocomplete="off"
    autocapitalize="off"
  ></textarea>
  <p class="hint">Plain text for now · spaces ignored</p>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button class="primary" disabled={!canSubmit} onclick={submit}>+ Add account</button>
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

  .field-label {
    font-size: 11px;
    color: #8b8f96;
    margin: 8px 2px 4px;
  }
  .text,
  .secret {
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
  .text:focus,
  .secret:focus {
    border-color: #5b8cff;
  }
  .text::placeholder,
  .secret::placeholder {
    color: #777b82;
  }
  .secret {
    font-family: "SF Mono", ui-monospace, "Menlo", monospace;
    letter-spacing: 0.04em;
    resize: none;
    line-height: 1.5;
  }
  .hint {
    font-size: 11px;
    color: #777b82;
    margin: 5px 2px 0;
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
