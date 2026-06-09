// The single typed boundary to the Rust command surface (`src-tauri/src/lib.rs`). Every
// screen calls these thin wrappers instead of hand-typing `invoke<…>("cmd")` generics, so
// the payload shapes — which mirror the `#[derive(Serialize)]` structs in `lib.rs` — live in
// exactly one place. Error handling stays at the call sites (the per-screen try/catch around
// these); the wrappers only name the command and type its result. The test mock
// (`src/test/backend-mock.ts`) implements the same command names + args against this contract.
import { invoke } from "@tauri-apps/api/core";

// --- Shared payload types (canonical TS mirrors of the Rust structs) ---

/** `encryption_status` — drives the unlock gate and the Security row's On/Off display. */
export type EncryptionStatus = { enabled: boolean; locked: boolean };

/** Secret-free account projection out of `list_accounts` (no secret crosses the bridge, D4). */
export type AccountView = { name: string };

/** E1 codes-list projection: the live OTP per account plus the shared period boundary. */
export type CodeView = {
  name: string;
  code: string;
  period_seconds: number;
  valid_until_unix: number;
};

/** Honest, count-only import result (no secret crosses the bridge, D4). */
export type ImportSummary = { added: number; skipped: number; relabeled: number };

// --- Codes list (E1) ---

export const getCodes = () => invoke<CodeView[]>("get_codes");

/** Content-size the popover window to the measured natural height. */
export const resizeMain = (height: number) => invoke<void>("resize_main", { height });

// --- Accounts ---

export const listAccounts = () => invoke<AccountView[]>("list_accounts");
export const addAccount = (name: string, secret: string) =>
  invoke<AccountView>("add_account", { name, secret });
export const renameAccount = (name: string, newName: string) =>
  invoke<void>("rename_account", { name, newName });
export const deleteAccount = (name: string) => invoke<void>("delete_account", { name });

// --- Encryption (UNIFIED_PLAN §3.4 E4, D7) ---

export const encryptionStatus = () => invoke<EncryptionStatus>("encryption_status");
export const setPassword = (next: string) => invoke<void>("set_password", { new: next });
export const changePassword = (current: string, next: string) =>
  invoke<void>("change_password", { old: current, new: next });
export const disablePassword = () => invoke<void>("disable_password");
export const unlock = (password: string) => invoke<void>("unlock", { password });

// --- Window / dialog chrome ---

/** Suspend the popover's focus-loss auto-hide while a native file sheet is in front. */
export const setDialogOpen = (open: boolean) => invoke<void>("set_dialog_open", { open });

// --- Backup / import (E6 / D6 / D11) ---

export const exportBackup = (destPath: string, password: string | null) =>
  invoke<void>("export_backup", { destPath, password });
export const importBackup = (srcPath: string, password: string | null) =>
  invoke<ImportSummary>("import_backup", { srcPath, password });
