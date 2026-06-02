<script lang="ts">
  // Load the global design tokens + shared primitives once for every route
  // (UNIFIED_PLAN D9). Screen `<style>` blocks consume the `--…` custom properties.
  import "../app.css";

  let { children } = $props();
</script>

{@render children()}
