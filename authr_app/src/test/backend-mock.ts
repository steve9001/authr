import { vi } from "vitest";

// In-memory stand-in for the Rust command surface (UNIFIED_PLAN §9.1 seam table).
// Mirrors `authr_app/src-tauri/src/lib.rs` closely enough to exercise the UI:
// duplicate-name + invalid-secret rejection (thrown as a string, like the real
// `map_err(|e| e.to_string())`), name mutation on rename, removal on delete, and
// a secret-free `AccountView` ({ name, issuer }) projection out of `list_accounts`.
export type AccountView = { name: string; issuer: string | null };

let accounts: AccountView[] = [];

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
