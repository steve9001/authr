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

import { invoke, setAccounts } from "../../test/backend-mock";
import SettingsPage from "./+page.svelte";

// Number of times `invoke` was called with a given command.
function callCount(cmd: string): number {
  return (invoke.mock.calls as unknown[][]).filter((c) => c[0] === cmd).length;
}

beforeEach(() => {
  setAccounts([
    { name: "alice" },
    { name: "bob" },
  ]);
});

describe("Settings manage page (§9.1 scenarios 5–10)", () => {
  // Scenario 5 + 10: rename happy path — ✎ opens a focused input prefilled with
  // the current name; Enter commits via rename_account and the list re-renders
  // off a follow-up list_accounts.
  it("renames a row via a focused input, then re-fetches the list", async () => {
    const { container } = render(SettingsPage);
    await screen.findByText("alice");
    const mountFetches = callCount("list_accounts");

    await fireEvent.click(screen.getAllByTitle("Rename")[0]); // first row = alice

    const input = (await waitFor(() => {
      const el = container.querySelector(".rename-input") as HTMLInputElement | null;
      expect(el).not.toBeNull();
      return el!;
    })) as HTMLInputElement;
    expect(input.value).toBe("alice");
    expect(document.activeElement).toBe(input);

    await fireEvent.input(input, { target: { value: "alice-2" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(screen.getByText("alice-2")).toBeInTheDocument());
    expect(invoke).toHaveBeenCalledWith("rename_account", {
      name: "alice",
      newName: "alice-2",
    });
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
    // Re-fetched after the mutation (scenario 10).
    expect(callCount("list_accounts")).toBe(mountFetches + 1);
  });

  // Scenario 6: Esc cancels the edit without calling rename_account.
  it("cancels a rename on Escape without invoking rename_account", async () => {
    const { container } = render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getAllByTitle("Rename")[0]);
    const input = (await waitFor(() => {
      const el = container.querySelector(".rename-input");
      expect(el).not.toBeNull();
      return el as HTMLInputElement;
    })) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "whatever" } });
    await fireEvent.keyDown(input, { key: "Escape" });

    await waitFor(() =>
      expect(container.querySelector(".rename-input")).toBeNull(),
    );
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith(
      "rename_account",
      expect.anything(),
    );
  });

  // Scenario 7: a collision keeps the row in edit mode and shows .rename-error.
  it("keeps edit mode and shows .rename-error on a name collision", async () => {
    const { container } = render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getAllByTitle("Rename")[0]); // alice
    const input = (await waitFor(() => {
      const el = container.querySelector(".rename-input");
      expect(el).not.toBeNull();
      return el as HTMLInputElement;
    })) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "bob" } }); // collides
    await fireEvent.keyDown(input, { key: "Enter" });

    const err = await waitFor(() => {
      const el = container.querySelector(".rename-error");
      expect(el).not.toBeNull();
      return el as HTMLElement;
    });
    expect(err.textContent).toContain("already exists");
    // Still editing — the input is still mounted.
    expect(container.querySelector(".rename-input")).not.toBeNull();
  });

  // Scenario 8 + D4: the delete modal shows the no-recovery copy and leaks no
  // secret; Cancel closes it with no invoke.
  it("shows the no-recovery modal with no secret, and Cancel makes no call", async () => {
    const { container } = render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getAllByTitle("Delete")[0]); // alice

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("no recovery");
    expect(dialog).toHaveTextContent("Delete");

    // D4: there is no secret in scope to leak — no secret field, and nothing in
    // the DOM looks like a base32 secret (a run of 16+ base32 chars).
    expect(container.querySelector("input.secret, textarea, .secret")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(/[A-Z2-7]{16,}/);

    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "delete_account",
      expect.anything(),
    );
  });

  // Scenario 9 + 10: confirming delete calls delete_account, re-fetches, and the
  // row disappears.
  it("deletes on confirm, then re-fetches and drops the row", async () => {
    render(SettingsPage);
    await screen.findByText("alice");
    const mountFetches = callCount("list_accounts");

    await fireEvent.click(screen.getAllByTitle("Delete")[0]); // alice
    await screen.findByRole("dialog");
    await fireEvent.click(screen.getByRole("button", { name: "🗑 Delete" }));

    await waitFor(() =>
      expect(screen.queryByText("alice")).not.toBeInTheDocument(),
    );
    expect(invoke).toHaveBeenCalledWith("delete_account", { name: "alice" });
    expect(callCount("list_accounts")).toBe(mountFetches + 1);
    expect(screen.getByText("bob")).toBeInTheDocument();
  });
});
