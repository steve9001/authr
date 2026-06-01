import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

// Seal the §9.1 seams (the page only needs core + navigation; the rest are stubbed for parity).
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
import { invoke, setAccounts, setEncryption } from "../../../test/backend-mock";
import PasswordPage from "./+page.svelte";

beforeEach(() => {
  setAccounts([]);
});

async function typeInto(el: Element, value: string) {
  await fireEvent.input(el, { target: { value } });
}

describe("Password screen — set mode (E4)", () => {
  // Not yet encrypted: shows the unrecoverable warning + new/confirm only (no current field).
  it("shows the unrecoverable warning and no current-password field", async () => {
    render(PasswordPage);
    expect(
      await screen.findByText(/your accounts can't be recovered/i),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("Current password")).not.toBeInTheDocument();
    expect(screen.getByLabelText("New password")).toBeInTheDocument();
    expect(screen.getByLabelText("Confirm password")).toBeInTheDocument();
  });

  // Happy path: set_password({ new }) is called and we navigate back to Settings.
  it("calls set_password with the new password and navigates to /settings", async () => {
    render(PasswordPage);
    await screen.findByLabelText("New password");
    await typeInto(screen.getByLabelText("New password"), "s3cret-pw");
    await typeInto(screen.getByLabelText("Confirm password"), "s3cret-pw");

    await fireEvent.click(screen.getByRole("button", { name: "Set password" }));

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/settings"));
    expect(invoke).toHaveBeenCalledWith("set_password", { new: "s3cret-pw" });
  });

  // Mismatched confirm: inline error, no backend call, no navigation.
  it("blocks on a confirm mismatch without calling set_password", async () => {
    render(PasswordPage);
    await screen.findByLabelText("New password");
    await typeInto(screen.getByLabelText("New password"), "abc12345");
    await typeInto(screen.getByLabelText("Confirm password"), "different");

    await fireEvent.click(screen.getByRole("button", { name: "Set password" }));

    expect(await screen.findByText("Passwords don't match")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("set_password", expect.anything());
    expect(goto).not.toHaveBeenCalled();
  });

  // Submit gating: the primary button is disabled until new + confirm are both filled.
  it("disables the button until new and confirm are filled", async () => {
    render(PasswordPage);
    const btn = await screen.findByRole("button", { name: "Set password" });
    expect(btn).toBeDisabled();
    await typeInto(screen.getByLabelText("New password"), "abc12345");
    expect(btn).toBeDisabled(); // confirm still empty
    await typeInto(screen.getByLabelText("Confirm password"), "abc12345");
    expect(btn).toBeEnabled();
  });
});

describe("Password screen — change mode (E4)", () => {
  beforeEach(() => {
    setAccounts([]);
    setEncryption({ password: "old-pw", locked: false });
  });

  // Already encrypted: the current-password field appears; the heading/button say "Change".
  it("shows a current-password field in change mode", async () => {
    render(PasswordPage);
    expect(await screen.findByLabelText("Current password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Update password" })).toBeInTheDocument();
  });

  // Happy path: change_password({ old, new }) then navigate back to Settings.
  it("calls change_password with old + new and navigates", async () => {
    render(PasswordPage);
    await screen.findByLabelText("Current password");
    await typeInto(screen.getByLabelText("Current password"), "old-pw");
    await typeInto(screen.getByLabelText("New password"), "new-pw-123");
    await typeInto(screen.getByLabelText("Confirm password"), "new-pw-123");

    await fireEvent.click(screen.getByRole("button", { name: "Update password" }));

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/settings"));
    expect(invoke).toHaveBeenCalledWith("change_password", {
      old: "old-pw",
      new: "new-pw-123",
    });
  });

  // Wrong current password: backend rejects → inline error, no navigation, stays on the page.
  it("surfaces a wrong current password inline and does not navigate", async () => {
    render(PasswordPage);
    await screen.findByLabelText("Current password");
    await typeInto(screen.getByLabelText("Current password"), "WRONG");
    await typeInto(screen.getByLabelText("New password"), "new-pw-123");
    await typeInto(screen.getByLabelText("Confirm password"), "new-pw-123");

    await fireEvent.click(screen.getByRole("button", { name: "Update password" }));

    expect(await screen.findByText("Incorrect password")).toBeInTheDocument();
    expect(goto).not.toHaveBeenCalled();
  });
});
