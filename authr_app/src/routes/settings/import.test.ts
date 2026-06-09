import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

// §9.1 seams + the Phase 5 dialog plugin (open() picks the import file).
vi.mock("@tauri-apps/api/core", async () => ({
  invoke: (await import("../../test/backend-mock")).invoke,
}));
vi.mock("@tauri-apps/api/window", async () => ({
  getCurrentWindow: (await import("../../test/backend-mock")).getCurrentWindow,
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", async () => ({
  writeText: (await import("../../test/backend-mock")).writeText,
}));
vi.mock("@tauri-apps/plugin-dialog", async () => ({
  save: (await import("../../test/backend-mock")).save,
  open: (await import("../../test/backend-mock")).open,
}));
vi.mock("@tauri-apps/api/path", async () => {
  const m = await import("../../test/backend-mock");
  return { downloadDir: m.downloadDir, homeDir: m.homeDir, join: m.join };
});
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

import { invoke, open, setAccounts, setImport } from "../../test/backend-mock";
import SettingsPage from "./+page.svelte";

beforeEach(() => {
  setAccounts([{ name: "alice" }]);
});

describe("Import accounts (D11)", () => {
  // The honest caveat is always present in the Backup section.
  it("words the additive-union caveat", async () => {
    render(SettingsPage);
    const caveat = await screen.findByText(/never deletes/i);
    expect(caveat).toHaveTextContent(/bring back an account you deleted/i);
  });

  // Happy path: open dialog → import_backup(null) → result toast → list re-fetch.
  it("imports a plaintext backup and shows a count toast", async () => {
    setImport({ result: { added: 9, skipped: 0, relabeled: 0 } });
    render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getByRole("button", { name: /Import accounts/ }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("Imported 9 new accounts"),
    );
    // Anchored in Downloads, and the focus-loss auto-hide is suspended then resumed.
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: "/Users/test/Downloads" }),
    );
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: true });
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: false });
    expect(invoke).toHaveBeenCalledWith("import_backup", {
      srcPath: "/tmp/import.authr",
      password: null,
    });
  });

  // A no-op import (everything already present) gets an honest toast.
  it("reports 'Nothing new' when the import only skips", async () => {
    setImport({ result: { added: 0, skipped: 3, relabeled: 0 } });
    render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getByRole("button", { name: /Import accounts/ }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("Nothing new to import"),
    );
  });

  // Encrypted file: a null-password import prompts; the right password then succeeds.
  it("prompts for the file's password when the backup is encrypted", async () => {
    setImport({
      result: { added: 2, skipped: 0, relabeled: 1 },
      encrypted: true,
      password: "file-pw",
    });
    render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getByRole("button", { name: /Import accounts/ }));

    // First attempt (null password) opens the prompt rather than toasting.
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Encrypted backup");
    const pw = screen.getByPlaceholderText("Backup password") as HTMLInputElement;

    // Wrong password surfaces inline, keeps the prompt open.
    await fireEvent.input(pw, { target: { value: "nope" } });
    await fireEvent.click(screen.getByRole("button", { name: "Import" }));
    expect(await screen.findByText("Incorrect password")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    // Right password imports, closes the prompt, and toasts the counts.
    await fireEvent.input(pw, { target: { value: "file-pw" } });
    await fireEvent.click(screen.getByRole("button", { name: "Import" }));

    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(screen.getByRole("status")).toHaveTextContent("Imported 2 new accounts");
    expect(screen.getByRole("status")).toHaveTextContent("1 relabeled");
  });

  // Cancelling the open dialog (null path) is a clean no-op — no import_backup call.
  it("does nothing when the open dialog is cancelled", async () => {
    open.mockResolvedValueOnce(null);
    render(SettingsPage);
    await screen.findByText("alice");

    await fireEvent.click(screen.getByRole("button", { name: /Import accounts/ }));

    await waitFor(() => expect(open).toHaveBeenCalled());
    // The guard is cleared even on cancel (the `finally`), so the popover keeps auto-hiding.
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: false });
    expect(invoke).not.toHaveBeenCalledWith("import_backup", expect.anything());
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
