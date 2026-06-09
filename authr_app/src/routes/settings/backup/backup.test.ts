import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

// §9.1 seams + the Phase 5 dialog plugin (save() picks the export path).
vi.mock("@tauri-apps/api/core", async () => ({
  invoke: (await import("../../../test/backend-mock")).invoke,
}));
vi.mock("@tauri-apps/api/window", async () => ({
  getCurrentWindow: (await import("../../../test/backend-mock")).getCurrentWindow,
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", async () => ({
  writeText: (await import("../../../test/backend-mock")).writeText,
}));
vi.mock("@tauri-apps/plugin-dialog", async () => ({
  save: (await import("../../../test/backend-mock")).save,
  open: (await import("../../../test/backend-mock")).open,
}));
vi.mock("@tauri-apps/api/path", async () => {
  const m = await import("../../../test/backend-mock");
  return { downloadDir: m.downloadDir, homeDir: m.homeDir, join: m.join };
});
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

import { goto } from "$app/navigation";
import {
  invoke,
  save,
  downloadDir,
  homeDir,
  setAccounts,
} from "../../../test/backend-mock";
import BackupPage from "./+page.svelte";

beforeEach(() => {
  setAccounts([{ name: "alice" }]);
});

async function typeInto(el: Element, value: string) {
  await fireEvent.input(el, { target: { value } });
}

describe("Backup screen (E6 / D6)", () => {
  // Encrypted export: password+confirm → save dialog → export_backup with that password.
  it("exports with the backup's own password and navigates back", async () => {
    render(BackupPage);
    await typeInto(await screen.findByLabelText("Backup password"), "copy-pw");
    await typeInto(screen.getByLabelText("Confirm password"), "copy-pw");

    await fireEvent.click(
      screen.getByRole("button", { name: "Save encrypted backup" }),
    );

    await waitFor(() => expect(goto).toHaveBeenCalledWith("/settings"));
    // The picker is anchored in Downloads with the filename prefilled.
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "/Users/test/Downloads/authr-vault.authr",
      }),
    );
    // The focus-loss auto-hide is suspended for the picker, then resumed.
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: true });
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: false });
    expect(invoke).toHaveBeenCalledWith("export_backup", {
      destPath: "/tmp/authr-vault.authr",
      password: "copy-pw",
    });
  });

  // Downloads can't be resolved on every system → fall back to the home dir.
  it("anchors the picker at the home dir when Downloads can't be resolved", async () => {
    downloadDir.mockRejectedValueOnce(new Error("no Downloads"));
    render(BackupPage);
    await typeInto(await screen.findByLabelText("Backup password"), "copy-pw");
    await typeInto(screen.getByLabelText("Confirm password"), "copy-pw");

    await fireEvent.click(
      screen.getByRole("button", { name: "Save encrypted backup" }),
    );

    await waitFor(() => expect(homeDir).toHaveBeenCalled());
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: "/Users/test/authr-vault.authr" }),
    );
  });

  // Plaintext path: no password ⇒ the button is gated behind the explicit confirmation.
  it("requires the plain-text confirmation before a plaintext export", async () => {
    render(BackupPage);
    const btn = await screen.findByRole("button", {
      name: "Save plain-text backup",
    });
    expect(btn).toBeDisabled();

    await fireEvent.click(screen.getByRole("checkbox"));
    expect(btn).toBeEnabled();

    await fireEvent.click(btn);
    await waitFor(() => expect(goto).toHaveBeenCalledWith("/settings"));
    // null password ⇒ Rust writes plaintext JSON.
    expect(invoke).toHaveBeenCalledWith("export_backup", {
      destPath: "/tmp/authr-vault.authr",
      password: null,
    });
  });

  // A confirm mismatch blocks before the save dialog even opens.
  it("blocks on a password/confirm mismatch", async () => {
    render(BackupPage);
    await typeInto(await screen.findByLabelText("Backup password"), "one");
    await typeInto(screen.getByLabelText("Confirm password"), "two");

    await fireEvent.click(
      screen.getByRole("button", { name: "Save encrypted backup" }),
    );

    expect(await screen.findByText("Passwords don't match")).toBeInTheDocument();
    expect(save).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith("export_backup", expect.anything());
  });

  // Cancelling the save dialog (null path) is a clean no-op.
  it("does nothing when the save dialog is cancelled", async () => {
    save.mockResolvedValueOnce(null);
    render(BackupPage);
    await typeInto(await screen.findByLabelText("Backup password"), "copy-pw");
    await typeInto(screen.getByLabelText("Confirm password"), "copy-pw");

    await fireEvent.click(
      screen.getByRole("button", { name: "Save encrypted backup" }),
    );

    await waitFor(() => expect(save).toHaveBeenCalled());
    // The guard is still cleared on the cancel path (the `finally`), so the popover
    // doesn't get stuck never auto-hiding.
    expect(invoke).toHaveBeenCalledWith("set_dialog_open", { open: false });
    expect(invoke).not.toHaveBeenCalledWith("export_backup", expect.anything());
    expect(goto).not.toHaveBeenCalled();
  });
});
