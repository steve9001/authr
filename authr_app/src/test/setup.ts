import "@testing-library/jest-dom/vitest";

// `@testing-library/svelte/vite` (svelteTesting) auto-unmounts components after
// each test. We additionally reset all mock call history between tests so the
// shared `invoke`/`goto` spies start clean per scenario.
import { afterEach, vi } from "vitest";

afterEach(() => {
  vi.clearAllMocks();
});
