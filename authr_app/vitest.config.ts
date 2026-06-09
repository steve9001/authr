import { defineConfig } from "vitest/config";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";

// Standalone config for the Tier 1 Svelte component tests (UNIFIED_PLAN §9.1).
// Deliberately separate from the Tauri `vite.config.js`: no fixed dev port, no
// SvelteKit plugin — the tray/positioner/window chrome never load here. We mount
// the page components directly in jsdom against a fully mocked backend.
export default defineConfig({
  plugins: [
    svelte({ hot: false, preprocess: vitePreprocess() }),
    svelteTesting(),
  ],
  resolve: {
    // The components import `$app/navigation` (a SvelteKit virtual module that
    // doesn't resolve without the kit plugin). Alias it to a local stub so the
    // import resolves; tests `vi.mock` it to assert navigation targets.
    alias: {
      "$app/navigation": new URL(
        "./src/test/app-navigation-stub.ts",
        import.meta.url,
      ).pathname,
      // `$lib` is a SvelteKit-provided alias; without the kit plugin here we map it
      // ourselves so shared components (e.g. $lib/Modal.svelte) resolve under jsdom.
      $lib: new URL("./src/lib", import.meta.url).pathname,
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{js,ts}"],
  },
});
