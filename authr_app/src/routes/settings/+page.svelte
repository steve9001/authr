<script lang="ts">
  import { goto } from "$app/navigation";
</script>

<!-- Stub Settings hub — the real E3 (Security / Backup / Accounts) lands in Phase 3. -->
<main>
  <header>
    <button class="back" onclick={() => goto("/")} title="Back">←</button>
    <h1>Settings</h1>
  </header>
  <p class="soon">Account management, encryption, and backup are coming next.</p>
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
    height: 100vh;
    box-sizing: border-box;
    padding: 8px 10px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
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
  .soon {
    color: #777b82;
    font-size: 13px;
    margin-top: 20px;
  }
</style>
