// Register a window-level Escape handler and return its unsubscribe, so each screen passes
// just the one line that varies (what Escape does) instead of re-implementing the
// addEventListener / branch-on-"Escape" / removeEventListener lifecycle. Call inside
// `onMount` and either `return` the unsubscribe (screens whose only teardown is this) or
// invoke it from a composite cleanup (the codes screen, which also tears down timers/listeners).
export function onEscape(handler: () => void): () => void {
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") handler();
  };
  window.addEventListener("keydown", onKey);
  return () => window.removeEventListener("keydown", onKey);
}
