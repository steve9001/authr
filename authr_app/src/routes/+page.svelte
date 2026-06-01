<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { goto } from "$app/navigation";

  type CodeView = {
    name: string;
    issuer: string | null;
    code: string;
    period_seconds: number;
    valid_until_unix: number;
  };

  let codes = $state<CodeView[]>([]);
  let filter = $state("");
  let nowMs = $state(Date.now());
  let copiedName = $state<string | null>(null);
  let locked = $state(false);
  let searchEl: HTMLInputElement | undefined;
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  // The unlock gate (UNIFIED_PLAN §3.4): if the store is encrypted+locked, divert to /unlock
  // before touching codes. Returns true when locked (and navigation was kicked off).
  async function gateIfLocked(): Promise<boolean> {
    try {
      const s = await invoke<{ enabled: boolean; locked: boolean }>(
        "encryption_status",
      );
      if (s.enabled && s.locked) {
        locked = true;
        goto("/unlock");
        return true;
      }
    } catch (e) {
      console.error("encryption_status failed", e);
    }
    return false;
  }

  async function refresh() {
    if (locked) return;
    try {
      codes = await invoke<CodeView[]>("get_codes");
    } catch (e) {
      console.error("get_codes failed", e);
      codes = [];
    }
  }

  // Substring filter on name (+issuer), case-insensitive — immediate, no debounce.
  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return codes;
    return codes.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        (c.issuer ?? "").toLowerCase().includes(q),
    );
  });

  // Single global countdown — every code shares the same 30s boundary, so the first
  // code's validity drives the whole bar (UNIFIED_PLAN E1: one global timer).
  const periodSeconds = $derived(codes[0]?.period_seconds ?? 30);
  const validUntil = $derived(codes[0]?.valid_until_unix ?? 0);
  const remaining = $derived(Math.max(0, validUntil - nowMs / 1000));
  const fraction = $derived(
    periodSeconds > 0 ? Math.min(1, remaining / periodSeconds) : 0,
  );
  const secondsLeft = $derived(Math.ceil(remaining));

  function grouped(code: string): string {
    return code.length === 6 ? `${code.slice(0, 3)} ${code.slice(3)}` : code;
  }

  async function copy(c: CodeView) {
    try {
      await writeText(c.code);
      copiedName = c.name;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copiedName = null), 1000);
    } catch (e) {
      console.error("clipboard write failed", e);
    }
  }

  onMount(() => {
    // Gate on the lock state first; only load codes if the store is open.
    gateIfLocked().then((diverted) => {
      if (!diverted) {
        refresh();
        searchEl?.focus();
      }
    });

    // Drive the bar and re-fetch on the real period rollover (not a client-side guess).
    const tick = setInterval(() => {
      nowMs = Date.now();
      if (validUntil && nowMs / 1000 >= validUntil) {
        refresh();
      }
    }, 250);

    // Refresh + refocus the filter each time the popover reopens — re-checking the lock
    // gate too, in case the session ended while hidden.
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        gateIfLocked().then((diverted) => {
          if (!diverted) {
            refresh();
            searchEl?.focus();
          }
        });
      }
    });

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") win.hide();
    };
    window.addEventListener("keydown", onKey);

    return () => {
      clearInterval(tick);
      unlisten.then((u) => u());
      window.removeEventListener("keydown", onKey);
      clearTimeout(copyTimer);
    };
  });
</script>

<main>
  <!-- Countdown bar — single global timer for when all codes roll. -->
  <div class="countdown">
    <div class="track">
      <div class="fill" style="width: {fraction * 100}%"></div>
    </div>
    <span class="secs">{secondsLeft}s</span>
  </div>

  <!-- Search + gear bar. -->
  <div class="searchbar">
    <input
      bind:this={searchEl}
      bind:value={filter}
      class="search"
      type="text"
      placeholder="Search…"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
    />
    <button class="gear" title="Settings" onclick={() => goto("/settings")}>
      ⚙
    </button>
  </div>

  <!-- Account list — whole row is the tap-to-copy target. -->
  <div class="list">
    {#if filtered.length === 0}
      <p class="empty">
        {codes.length === 0 ? "No accounts yet" : "No matches"}
      </p>
    {:else}
      {#each filtered as c (c.name)}
        <button class="row" onclick={() => copy(c)}>
          <span class="name">{c.name}</span>
          {#if copiedName === c.name}
            <span class="copied">✓ copied!</span>
          {:else}
            <span class="code">{grouped(c.code)}</span>
          {/if}
        </button>
      {/each}
    {/if}
  </div>
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
    overflow: hidden;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 8px 10px;
    box-sizing: border-box;
    gap: 8px;
  }

  /* Countdown bar */
  .countdown {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .track {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: #34373d;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: #5b8cff;
    border-radius: 2px;
    transition: width 0.25s linear;
  }
  .secs {
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    color: #8b8f96;
    min-width: 26px;
    text-align: right;
  }

  /* Search + gear */
  .searchbar {
    display: flex;
    gap: 6px;
  }
  .search {
    flex: 1;
    min-width: 0;
    background: #34373d;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #e6e7e9;
    padding: 6px 9px;
    font-size: 13px;
    outline: none;
  }
  .search:focus {
    border-color: #5b8cff;
  }
  .search::placeholder {
    color: #777b82;
  }
  .gear {
    width: 32px;
    background: #34373d;
    border: none;
    border-radius: 6px;
    color: #c7c9cd;
    font-size: 15px;
    cursor: pointer;
  }
  .gear:hover {
    background: #3e424a;
  }

  /* Account list */
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 9px 10px;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: #2a2d33;
  }
  .row:active {
    background: #34373d;
  }
  .name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 10px;
  }
  .code {
    font-family: "SF Mono", ui-monospace, "Menlo", monospace;
    font-size: 15px;
    letter-spacing: 0.06em;
    color: #f3f4f6;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .copied {
    font-size: 13px;
    color: #4ec98a;
    white-space: nowrap;
  }
  .empty {
    color: #777b82;
    font-size: 13px;
    text-align: center;
    margin-top: 24px;
  }
</style>
