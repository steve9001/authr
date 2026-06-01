import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

vi.mock("@tauri-apps/api/core", async () => ({
  invoke: (await import("../../test/backend-mock")).invoke,
}));
vi.mock("@tauri-apps/api/window", async () => ({
  getCurrentWindow: (await import("../../test/backend-mock")).getCurrentWindow,
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", async () => ({
  writeText: (await import("../../test/backend-mock")).writeText,
}));
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

import { goto } from "$app/navigation";
import { invoke, setAccounts, setEncryption } from "../../test/backend-mock";
import UnlockPage from "./+page.svelte";

beforeEach(() => {
  setAccounts([]);
  // Encrypted + locked — the state under which the gate routes here.
  setEncryption({ password: "open-sesame", locked: true });
});

async function typeInto(el: Element, value: string) {
  await fireEvent.input(el, { target: { value } });
}

describe("Unlock gate (§3.4)", () => {
  it("shows the locked prompt with submit disabled until a password is typed", async () => {
    render(UnlockPage);
    expect(screen.getByText("Authr is locked")).toBeInTheDocument();
    const btn = screen.getByRole("button", { name: "Unlock" });
    expect(btn).toBeDisabled();
    await typeInto(screen.getByPlaceholderText("Password"), "x");
    expect(btn).toBeEnabled();
  });

  // Correct password: unlock({ password }) is called and we route to the main list.
  it("unlocks and navigates to / on the correct password", async () => {
    render(UnlockPage);
    await typeInto(screen.getByPlaceholderText("Password"), "open-sesame");
    await fireEvent.click(screen.getByRole("button", { name: "Unlock" }));

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/"));
    expect(invoke).toHaveBeenCalledWith("unlock", { password: "open-sesame" });
  });

  // Wrong password: inline error, no navigation, the field is cleared for a retry.
  it("shows an error and stays put on a wrong password", async () => {
    render(UnlockPage);
    const input = screen.getByPlaceholderText("Password") as HTMLInputElement;
    await typeInto(input, "nope");
    await fireEvent.click(screen.getByRole("button", { name: "Unlock" }));

    expect(await screen.findByText("Incorrect password")).toBeInTheDocument();
    expect(goto).not.toHaveBeenCalled();
    await waitFor(() => expect(input.value).toBe(""));
  });
});
