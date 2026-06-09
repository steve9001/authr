import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

vi.mock("@tauri-apps/api/core", async () => ({
  invoke: (await import("../test/backend-mock")).invoke,
}));
vi.mock("@tauri-apps/api/window", async () => ({
  getCurrentWindow: (await import("../test/backend-mock")).getCurrentWindow,
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", async () => ({
  writeText: (await import("../test/backend-mock")).writeText,
}));
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

import { goto } from "$app/navigation";
import {
  invoke,
  writeText,
  setAccounts,
  setCodes,
  setEncryption,
  type CodeView,
} from "../test/backend-mock";
import Page from "./+page.svelte";

// A fixed wall clock so the countdown derivations are deterministic. NOW_SEC is the unix-seconds
// instant `valid_until_unix` is expressed against in these fixtures.
const NOW_MS = 1_700_000_000_000;
const NOW_SEC = NOW_MS / 1000;

function code(name: string, overrides: Partial<CodeView> = {}): CodeView {
  return {
    name,
    code: "123456",
    period_seconds: 30,
    valid_until_unix: NOW_SEC + 20,
    ...overrides,
  };
}

// Restored per test so a mocked clock never leaks into the next case. We restore THIS spy only
// (not vi.restoreAllMocks) so the shared backend-mock vi.fn implementations stay intact.
let nowSpy: ReturnType<typeof vi.spyOn> | undefined;

beforeEach(() => {
  setAccounts([]); // also resets codes + encryption to plaintext/unlocked
});

afterEach(() => {
  nowSpy?.mockRestore();
  nowSpy = undefined;
  vi.useRealTimers();
});

describe("E1 codes list — gating", () => {
  // Encrypted + locked: the gate diverts to /unlock before any code is read.
  it("diverts to /unlock and does not load codes when locked", async () => {
    setEncryption({ password: "open-sesame", locked: true });
    setCodes([code("GitHub")]);
    render(Page);

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/unlock"));
    expect(invoke).not.toHaveBeenCalledWith("get_codes");
  });

  // Unlocked (default): codes load and render.
  it("loads codes when the store is open", async () => {
    setCodes([code("GitHub")]);
    render(Page);

    expect(await screen.findByText("GitHub")).toBeInTheDocument();
    expect(goto).not.toHaveBeenCalledWith("/unlock");
    expect(invoke).toHaveBeenCalledWith("get_codes");
  });
});

describe("E1 codes list — filter", () => {
  beforeEach(() => {
    setCodes([code("GitHub"), code("GitLab"), code("AWS")]);
  });

  it("filters by case-insensitive name substring", async () => {
    render(Page);
    await screen.findByText("GitHub");

    await fireEvent.input(screen.getByPlaceholderText("Search…"), {
      target: { value: "GIT" },
    });

    expect(screen.getByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("GitLab")).toBeInTheDocument();
    expect(screen.queryByText("AWS")).not.toBeInTheDocument();
  });

  it("shows the 'No matches' empty state when nothing matches", async () => {
    render(Page);
    await screen.findByText("GitHub");

    await fireEvent.input(screen.getByPlaceholderText("Search…"), {
      target: { value: "zzz" },
    });

    expect(screen.getByText("No matches")).toBeInTheDocument();
  });
});

describe("E1 codes list — empty state", () => {
  it("shows 'No accounts yet' with no codes and hides the countdown", async () => {
    render(Page);
    expect(await screen.findByText("No accounts yet")).toBeInTheDocument();
    // The countdown bar is hidden when there are no accounts.
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("get_codes"));
    expect(document.querySelector(".countdown")).toBeNull();
  });
});

describe("E1 codes list — grouped() 3+3 split", () => {
  it("renders a 6-digit code as two space-separated triples", async () => {
    setCodes([code("GitHub", { code: "123456" })]);
    render(Page);
    expect(await screen.findByText("123 456")).toBeInTheDocument();
  });

  it("leaves a non-6-digit code unsplit", async () => {
    setCodes([code("Steam", { code: "ABCDE" })]);
    render(Page);
    expect(await screen.findByText("ABCDE")).toBeInTheDocument();
  });
});

describe("E1 codes list — copy to clipboard", () => {
  it("copies the raw code, shows the confirmation, then reverts after 1.2s", async () => {
    vi.useFakeTimers();
    setCodes([code("GitHub", { code: "123456" })]);
    render(Page);

    // Flush the onMount gate → get_codes chain (microtasks) without firing the interval.
    await vi.advanceTimersByTimeAsync(0);
    const row = screen.getByText("123 456").closest("button")!;

    await fireEvent.click(row);
    await vi.advanceTimersByTimeAsync(0);

    // Raw (ungrouped) code goes to the clipboard; the row swaps to the confirmation.
    expect(writeText).toHaveBeenCalledWith("123456");
    expect(screen.getByText("✓ copied!")).toBeInTheDocument();
    expect(screen.queryByText("123 456")).not.toBeInTheDocument();

    // The confirmation clears on the 1.2s timer, restoring the code.
    await vi.advanceTimersByTimeAsync(1200);
    expect(screen.queryByText("✓ copied!")).not.toBeInTheDocument();
    expect(screen.getByText("123 456")).toBeInTheDocument();
  });
});

describe("E1 codes list — countdown math", () => {
  it("derives secondsLeft and the fill fraction from the global boundary", async () => {
    nowSpy = vi.spyOn(Date, "now").mockReturnValue(NOW_MS);
    // 20s remaining of a 30s period.
    setCodes([code("GitHub", { period_seconds: 30, valid_until_unix: NOW_SEC + 20 })]);
    render(Page);

    // ceil(20) = 20 seconds shown.
    expect(await screen.findByText("20s")).toBeInTheDocument();
    // fraction = remaining / period = 20/30.
    const fill = document.querySelector(".fill") as HTMLElement;
    expect(fill.style.width).toBe(`${(20 / 30) * 100}%`);
  });

  it("clamps a past boundary to 0s and an empty bar", async () => {
    nowSpy = vi.spyOn(Date, "now").mockReturnValue(NOW_MS);
    setCodes([code("GitHub", { period_seconds: 30, valid_until_unix: NOW_SEC - 5 })]);
    render(Page);

    expect(await screen.findByText("0s")).toBeInTheDocument();
    const fill = document.querySelector(".fill") as HTMLElement;
    expect(fill.style.width).toBe("0%");
  });
});
