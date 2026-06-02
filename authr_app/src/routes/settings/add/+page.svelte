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
  /* Shared shell, header, fields + buttons come from app.css; only the secret
     textarea's monospace treatment and the hint line are unique to E5. */
  main {
    display: flex;
    flex-direction: column;
  }

  .secret {
    font-family: var(--font-mono);
    letter-spacing: 0.04em;
    resize: none;
    line-height: 1.5;
  }
  .hint {
    font-size: 11px;
    color: var(--text-faint);
    margin: 5px 2px 0;
  }
</style>
