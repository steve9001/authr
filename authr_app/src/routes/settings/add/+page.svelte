<script lang="ts">
  import { onMount, tick } from "svelte";
  import { goto } from "$app/navigation";
  import { onEscape } from "$lib/keys";
  import { addAccount } from "$lib/backend";

  let name = $state("");
  let secret = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let showSecretHelp = $state(false);
  let nameEl: HTMLInputElement | undefined;

  const canSubmit = $derived(name.trim().length > 0 && secret.trim().length > 0 && !busy);

  async function submit() {
    if (!canSubmit) return;
    busy = true;
    error = null;
    try {
      await addAccount(name.trim(), secret);
      goto("/settings");
    } catch (e) {
      // Surfaces duplicate-name / invalid-secret errors from core inline.
      error = String(e);
      busy = false;
    }
  }

  onMount(() => {
    tick().then(() => nameEl?.focus());
    return onEscape(() => goto("/settings"));
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

  <div class="label-row">
    <label class="field-label" for="acct-secret">Secret key</label>
    <button
      type="button"
      class="help-toggle"
      aria-label="Where do I find this?"
      aria-expanded={showSecretHelp}
      onclick={() => (showSecretHelp = !showSecretHelp)}>ⓘ</button>
  </div>
  {#if showSecretHelp}
    <p class="help">
      Setting up two-factor authentication usually shows a QR code to scan with a phone app. Look
      for an option near the QR code — often labeled “Can’t scan?”, “Enter code manually”, or “Show
      secret key” — to reveal the secret as text. Paste that string here.
    </p>
  {/if}
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
  <p class="hint">Spaces and capitalization don’t matter</p>

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
  .label-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .help-toggle {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-faint);
    font-size: 13px;
    line-height: 1;
  }
  .help-toggle:hover,
  .help-toggle[aria-expanded="true"] {
    color: var(--accent);
  }
  .help {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-dim);
    background: var(--control);
    border-radius: var(--radius-sm);
    padding: 9px 10px;
    margin: 6px 0 0;
  }
</style>
