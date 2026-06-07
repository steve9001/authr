import { vi } from "vitest";

// In-memory stand-in for the Rust command surface (UNIFIED_PLAN §9.1 seam table).
// Mirrors `authr_app/src-tauri/src/lib.rs` closely enough to exercise the UI:
// duplicate-name + invalid-secret rejection (thrown as a string, like the real
// `map_err(|e| e.to_string())`), name mutation on rename, removal on delete, and
// a secret-free `AccountView` ({ name, issuer }) projection out of `list_accounts`.
export type AccountView = { name: string; issuer: string | null };
export type ImportSummary = { added: number; skipped: number; relabeled: number };

let accounts: AccountView[] = [];

// Backup/import seam (UNIFIED_PLAN §5). Tier 1 mocks the backend, so we don't move real
// bytes through a file — we record the last export args for assertions and drive import via
// a configurable result / encrypted-file scenario.
let lastExport: { destPath: string; password: string | null } | null = null;
let importResult: ImportSummary = { added: 0, skipped: 0, relabeled: 0 };
// When set, importing this with a null password throws the "encrypted" error (so the UI
// prompts), and only `importPassword` decrypts it.
let importEncrypted = false;
let importPassword: string | null = null;

// Encryption state (UNIFIED_PLAN §3.3 Phase 4). `vaultPassword === null` ⇒ the store is
// plaintext (encryption disabled); a string ⇒ encrypted with that passphrase. `unlocked`
// mirrors the in-session passphrase hold (D7). Defaults: disabled + unlocked.
let vaultPassword: string | null = null;
let unlocked = true;

/** Reset the backing store; call in `beforeEach`. Also resets encryption to disabled. */
export function setAccounts(initial: AccountView[]): void {
  accounts = initial.map((a) => ({ ...a }));
  vaultPassword = null;
  unlocked = true;
  lastExport = null;
  importResult = { added: 0, skipped: 0, relabeled: 0 };
  importEncrypted = false;
  importPassword = null;
}

/** The args of the last `export_backup` call (for assertions). */
export function getLastExport(): { destPath: string; password: string | null } | null {
  return lastExport;
}

/**
 * Drive the import seam for a test. `result` is the `ImportSummary` a successful import
 * returns; `encrypted`/`password` make a null-password import throw the "encrypted" error
 * (so the UI prompts) and require `password` to succeed.
 */
export function setImport(opts: {
  result?: ImportSummary;
  encrypted?: boolean;
  password?: string;
}): void {
  if (opts.result) importResult = opts.result;
  importEncrypted = opts.encrypted ?? false;
  importPassword = opts.password ?? null;
}

/** Read the current store (for assertions). */
export function getAccounts(): AccountView[] {
  return accounts;
}

/**
 * Drive the encryption seam for a test. `password: null` (default) leaves the store
 * plaintext; a string encrypts it, and `locked` controls whether this session holds the
 * passphrase (locked ⇒ the `/unlock` gate).
 */
export function setEncryption(opts: { password?: string | null; locked?: boolean }): void {
  vaultPassword = opts.password ?? null;
  unlocked = vaultPassword === null ? true : !(opts.locked ?? false);
}

// Base32 (RFC 4648) after stripping whitespace — matches core's "spaces ignored"
// contract. An empty or non-base32 secret is rejected like core's `Invalid secret`.
function isValidSecret(secret: string): boolean {
  const stripped = secret.replace(/\s+/g, "");
  return stripped.length > 0 && /^[A-Z2-7]+=*$/i.test(stripped);
}

export const invoke = vi.fn(
  async (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
    switch (cmd) {
      case "list_accounts":
        return accounts.map((a) => ({ ...a }));

      case "add_account": {
        const name = args?.name as string;
        const secret = args?.secret as string;
        if (accounts.some((a) => a.name === name)) {
          throw `Account '${name}' already exists`;
        }
        if (!isValidSecret(secret)) {
          throw `Invalid secret: not valid base32`;
        }
        const view: AccountView = { name, issuer: null };
        accounts.push(view);
        return { ...view };
      }

      case "rename_account": {
        const name = args?.name as string;
        const newName = args?.newName as string;
        if (accounts.some((a) => a.name === newName)) {
          throw `Account '${newName}' already exists`;
        }
        const row = accounts.find((a) => a.name === name);
        if (!row) throw `Account '${name}' not found`;
        row.name = newName;
        return null;
      }

      case "delete_account": {
        const name = args?.name as string;
        accounts = accounts.filter((a) => a.name !== name);
        return null;
      }

      // --- Phase 4 encryption commands (UNIFIED_PLAN §3.3) ---
      case "encryption_status":
        return {
          enabled: vaultPassword !== null,
          locked: vaultPassword !== null && !unlocked,
        };

      case "set_password": {
        if (vaultPassword !== null) throw "Encryption is already enabled";
        vaultPassword = args?.new as string;
        unlocked = true;
        return null;
      }

      case "change_password": {
        const oldPw = args?.old as string;
        if (vaultPassword === null) throw "Encryption is not enabled";
        if (oldPw !== vaultPassword) throw "Incorrect password";
        vaultPassword = args?.new as string;
        unlocked = true;
        return null;
      }

      case "unlock": {
        const pw = args?.password as string;
        if (vaultPassword === null) return null; // nothing to unlock
        if (pw !== vaultPassword) throw "Incorrect password";
        unlocked = true;
        return null;
      }

      // Focus-loss auto-hide guard around a native file dialog (no-op in the mock; the test
      // asserts it's flipped true before and false after a picker).
      case "set_dialog_open":
        return null;

      // --- Phase 5 backup/import commands (UNIFIED_PLAN §3.3, §5) ---
      case "export_backup": {
        lastExport = {
          destPath: args?.destPath as string,
          password: (args?.password as string | null) ?? null,
        };
        return null;
      }

      case "import_backup": {
        const password = (args?.password as string | null) ?? null;
        if (importEncrypted && password === null) {
          throw "This backup is encrypted — enter its password";
        }
        if (importEncrypted && password !== importPassword) {
          throw "Incorrect password";
        }
        return { ...importResult };
      }

      default:
        throw `unexpected command: ${cmd}`;
    }
  },
);

// `@tauri-apps/api/window` — only `getCurrentWindow().hide()` is used (Escape).
export const hide = vi.fn();
export function getCurrentWindow() {
  return { hide };
}

// `@tauri-apps/plugin-clipboard-manager` — only E1 (the codes view) uses it; the
// Phase 3 settings pages don't, but it's harmless to stub for parity.
export const writeText = vi.fn();

// `@tauri-apps/plugin-dialog` — `save()` picks an export path (E6), `open()` picks an import
// file. Both default to a path; tests override with `mockResolvedValueOnce(null)` to model a
// cancelled dialog.
export const save = vi.fn(async (): Promise<string | null> => "/tmp/authr-vault.authr");
export const open = vi.fn(async (): Promise<string | null> => "/tmp/import.authr");

// `@tauri-apps/api/path` — the pickers anchor at a home-relative base dir (Downloads, falling
// back to home) and prefill the filename. The base-dir resolution is mockable so tests can
// drive the Downloads-rejects-→-home fallback.
export const downloadDir = vi.fn(async (): Promise<string> => "/Users/test/Downloads");
export const homeDir = vi.fn(async (): Promise<string> => "/Users/test");
export const join = vi.fn(async (...parts: string[]): Promise<string> => parts.join("/"));
