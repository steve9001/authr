// Resolution stub for SvelteKit's `$app/navigation` virtual module (aliased in
// vitest.config.ts). Tests override this via `vi.mock("$app/navigation", ...)`
// to install a spy and assert navigation targets; this default keeps the import
// resolvable if a test forgets to mock it.
export function goto(_url: string): Promise<void> {
  return Promise.resolve();
}
