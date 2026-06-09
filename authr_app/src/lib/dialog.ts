// Shared scaffolding for the two screens that open a native file sheet (backup export +
// account import). Both need the same two things around the picker, and both are subtle
// enough that copy-pasting them invites a future call site getting one wrong.
import { downloadDir, homeDir } from "@tauri-apps/api/path";
import { setDialogOpen } from "$lib/backend";

// Where a backup picker should be anchored: Downloads (the conventional "a file I just
// exported / am about to import" spot, one click from Finder's sidebar), falling back to
// the home dir on systems where Downloads can't be resolved. Callers that need a filename
// prefilled `join()` it onto this themselves.
export async function backupBaseDir(): Promise<string> {
  try {
    return await downloadDir();
  } catch {
    return await homeDir();
  }
}

// Run `fn` (the native save()/open() call) with the popover's focus-loss auto-hide
// suspended, then resume it no matter how `fn` settles — otherwise the popover hides on
// focus loss and tears the native sheet down with it. The `finally` is load-bearing: it
// must run on cancel and on error too, so the popover never gets stuck never auto-hiding.
export async function withDialogGuard<T>(fn: () => Promise<T>): Promise<T> {
  await setDialogOpen(true);
  try {
    return await fn();
  } finally {
    await setDialogOpen(false);
  }
}
