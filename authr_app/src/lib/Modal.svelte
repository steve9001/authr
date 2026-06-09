<script lang="ts">
  // Shared overlay/dialog shell (extracted from the settings + password screens, which
  // carried this verbatim). Encapsulates the scrim, the click-outside-to-dismiss, and the
  // role/aria-modal contract; the `.overlay`/`.modal`/`.modal-actions` styling lives in
  // app.css alongside the other shared primitives. Body content is passed as the default
  // children snippet; the button row goes through the `actions` snippet so it lands inside
  // the `.modal-actions` flex row.
  import type { Snippet } from "svelte";

  let {
    onclose,
    children,
    actions,
  }: {
    onclose: () => void;
    children: Snippet;
    actions: Snippet;
  } = $props();
</script>

<div
  class="overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="modal" role="dialog" aria-modal="true">
    {@render children()}
    <div class="modal-actions">{@render actions()}</div>
  </div>
</div>
