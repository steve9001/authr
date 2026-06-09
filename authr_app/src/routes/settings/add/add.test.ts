import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

// Seal all four §9.1 seams. The add page only imports `core` + `$app/navigation`,
// but window/clipboard are stubbed too for parity with the plan's seam table.
vi.mock("@tauri-apps/api/core", async () => ({
  invoke: (await import("../../../test/backend-mock")).invoke,
}));
vi.mock("@tauri-apps/api/window", async () => ({
  getCurrentWindow: (await import("../../../test/backend-mock")).getCurrentWindow,
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", async () => ({
  writeText: (await import("../../../test/backend-mock")).writeText,
}));
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

import { goto } from "$app/navigation";
import { invoke, setAccounts, getAccounts } from "../../../test/backend-mock";
import AddPage from "./+page.svelte";

beforeEach(() => {
  setAccounts([]);
});

async function typeInto(el: Element, value: string) {
  await fireEvent.input(el, { target: { value } });
}

describe("Add account page (§9.1 scenarios 1–4, 10)", () => {
  // Scenario 1 + 10: happy path forwards the trimmed name and the RAW spaced
  // secret (whitespace stripping is core's job), then navigates to /settings
  // (where E1/E3 re-fetch on mount).
  it("forwards trimmed name + raw spaced secret and navigates to /settings", async () => {
    render(AddPage);
    await typeInto(screen.getByLabelText("Account name"), "GitHub ");
    await typeInto(screen.getByLabelText("Secret key"), "JBSW Y3DP EHPK 3PXP");

    await fireEvent.click(screen.getByRole("button", { name: "+ Add account" }));

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/settings"));
    expect(invoke).toHaveBeenCalledWith("add_account", {
      name: "GitHub",
      secret: "JBSW Y3DP EHPK 3PXP",
    });
    // The store actually grew — proves the command ran, not just that goto fired.
    expect(getAccounts().map((a) => a.name)).toEqual(["GitHub"]);
  });

  // Scenario 2: duplicate name → inline .error, no navigation.
  it("renders an inline error and does not navigate on a duplicate name", async () => {
    setAccounts([{ name: "GitHub" }]);
    render(AddPage);
    await typeInto(screen.getByLabelText("Account name"), "GitHub");
    await typeInto(screen.getByLabelText("Secret key"), "JBSWY3DPEHPK3PXP");

    await fireEvent.click(screen.getByRole("button", { name: "+ Add account" }));

    const err = await screen.findByText("Account 'GitHub' already exists");
    expect(err).toHaveClass("error");
    expect(goto).not.toHaveBeenCalled();
  });

  // Scenario 3: invalid secret → inline .error.
  it("renders an inline error on an invalid secret", async () => {
    render(AddPage);
    await typeInto(screen.getByLabelText("Account name"), "Work");
    await typeInto(screen.getByLabelText("Secret key"), "INVALID!!!");

    await fireEvent.click(screen.getByRole("button", { name: "+ Add account" }));

    const err = await screen.findByText(/^Invalid secret:/);
    expect(err).toHaveClass("error");
    expect(goto).not.toHaveBeenCalled();
  });

  // Scenario 4: submit gating — the primary button is disabled until BOTH
  // name and secret are non-empty.
  it("disables the submit button until name and secret are both filled", async () => {
    render(AddPage);
    const btn = screen.getByRole("button", { name: "+ Add account" });
    expect(btn).toBeDisabled();

    await typeInto(screen.getByLabelText("Account name"), "Work");
    expect(btn).toBeDisabled(); // secret still empty

    await typeInto(screen.getByLabelText("Secret key"), "JBSWY3DPEHPK3PXP");
    expect(btn).toBeEnabled();

    await typeInto(screen.getByLabelText("Account name"), "   "); // whitespace-only name
    expect(btn).toBeDisabled();
  });
});
