# Phase 3 — Encrypt the on-disk cache

Goal: anyone with filesystem access to `data/favicons/<uid>/` should NOT be
able to enumerate the hostnames a user has saved Login ciphers for, nor
read the raw icon bytes.

## Threat model

| Attacker capability | Phase 1/2 | Phase 3 target |
|--|--|--|
| Read `data/favicons/<uid>/mail.google.com.png` | Sees the file exists and its bytes | Neither the filename nor the bytes reveal anything |
| Read `data/favicons/<uid>/index.json` | Full hostname list | Opaque blob |
| Read both the SQLite vault and the favicons dir | Already has ciphers | No new info from favicons |

The bar is parity with the SQLite vault at rest. The vault today is
encrypted only in memory (the SDK's keystore), so the SQLite file on disk
contains unencrypted metadata and encrypted-at-rest-by-SDK payloads. Our
target: filesystem contents are meaningless to an offline observer. We
don't try to protect against malware running as the user — that's out of
scope for a desktop vault.

## Decision: revisit SQLite vs. files-plus-sidecar

Phase 1 uses files + `index.json`. At Phase 3 we have two credible
options:

### Option A — Keep files, add encryption

- Each `<hostname>.png` is replaced by `<opaque_name>.bin` where
  `opaque_name = blake3(user_salt || hostname).to_hex()[..32]`.
- File contents: `nonce (12 bytes) || AES-256-GCM ciphertext` over the PNG
  bytes.
- `index.json` becomes `index.bin`, encrypted as a single blob with a
  different derived key.
- `user_salt` is a 32-byte random blob generated on first access, stored
  in a new `data/favicons/<uid>/.salt` file. If the salt is lost the cache
  is unrecoverable but self-heals — we just nuke the dir and re-fetch.

Pros: minimal dep additions (`aes-gcm` + `hkdf` + `blake3`, all small).
No SQLite learning curve. Filesystem primitive; cross-platform.

Cons: we own the crypto wiring. Mistakes in nonce handling or key
derivation are on us. `.salt` is a new object to forget about in
`evict_user`.

### Option B — Switch to a dedicated SQLite DB per user

- `data/favicons/<uid>.sqlite` — a separate DB, NOT the SDK's vault DB.
- Schema: `CREATE TABLE icons (host_hash BLOB PRIMARY KEY, encrypted_blob BLOB,
  fetched_at INTEGER, max_age_secs INTEGER, status INTEGER)`.
- Encryption via `sqlcipher` (the Rust binding is `rusqlite` with the
  `sqlcipher` feature, or `sqlcipher`-direct).

Pros: queries like "all stale entries" become SQL one-liners. Transaction
safety on bulk updates. The SDK already depends on `rusqlite` transitively
via `bitwarden_state`, so adding `rusqlite` as a direct dep at the desktop
level is cheap.

Cons: `sqlcipher` is a C dep (via `libsqlite3-sys` feature-flagged).
Bundled builds work but add binary size. Cross-compiling for all three
platforms adds CI complexity. Adds a runtime key-derivation + DB open on
every user access (or a single connection held open — lifecycle to manage).

### Option C — Use the SDK's `Repository<Favicon>` pattern

- Register a `Favicon` type with the SDK's `StateRegistry` and store it in
  the user's existing vault SQLite via `client.platform().state().get::<Favicon>()`.
- Leverages the SDK's encryption wrapping automatically.

Pros: no new storage code; deletion on `log_out` is handled by whatever
cleanup the SDK already does for vault data.

Cons: binds favicon lifetime to unlock state — we can't access favicons
while the vault is locked. But by definition, if the vault is locked, we
can't render cipher lists anyway, so we wouldn't be rendering favicons.
Worth re-examining. Also: the SDK's `Repository` pattern isn't designed
for bulk binary data — each `Favicon` would need a `RepositoryItem` impl,
and the SDK's wrapping would bloat the per-entry storage footprint.

### Recommendation

Decide at Phase 3 planning time based on how painful Phase 2's
`IndexFile` schema has become and whether we've grown other "opaque blob
per user" needs in the meantime. If Phase 2 is comfortable, stick with
**Option A** — minimum code delta, clear crypto story.

If meanwhile we've added a second piece of per-user state that would
benefit from SQL (e.g. a local search index, a custom-order list for the
vault), switch to **Option B** and bring favicons along. Don't go to
**Option C** unless the SDK itself starts exposing a general-purpose
encrypted KV for app-level use.

## Option A — concrete plan

### Crypto primitives

- `aes-gcm = "0.10"` — AEAD.
- `hkdf = "0.12"` — key derivation.
- `blake3 = "1"` — content-hash for filenames.
- `rand = "0.8"` — for the salt and per-write nonce.

One 32-byte random salt per user, written to
`data/favicons/<uid>/.salt` on first use. Two keys derived via HKDF:

```rust
const NAMING_INFO: &[u8] = b"bitwarden-desktop-next:favicon-naming:v1";
const CONTENT_INFO: &[u8] = b"bitwarden-desktop-next:favicon-content:v1";

fn derive_keys(salt: &[u8; 32]) -> (NamingKey, ContentKey) {
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(None, salt);
    let mut naming = [0u8; 32];
    let mut content = [0u8; 32];
    hkdf.expand(NAMING_INFO, &mut naming).unwrap();
    hkdf.expand(CONTENT_INFO, &mut content).unwrap();
    (NamingKey(naming), ContentKey(content))
}
```

### Naming

```rust
fn encrypted_filename(naming_key: &NamingKey, hostname: &str) -> String {
    let mut h = blake3::Hasher::new_keyed(&naming_key.0);
    h.update(hostname.as_bytes());
    let hex = h.finalize().to_hex();
    let mut out = String::with_capacity(36);
    out.push_str(&hex[..32]);
    out.push_str(".bin");
    out
}
```

Keyed Blake3 is a MAC — same hostname with the same salt produces the same
filename, enabling `get(hostname)` → path lookup. Different users produce
different filenames even for the same hostname because the salt differs.

### Content encryption

```rust
fn encrypt(key: &ContentKey, plaintext: &[u8]) -> Vec<u8> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    let cipher = Aes256Gcm::new(&key.0.into());
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill(&mut nonce);
    let ct = cipher.encrypt(&nonce.into(), plaintext).expect("encrypt can't fail");
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    out
}
```

### `index.bin`

Same scheme as content: encrypt a single `serde_json::to_vec(index)` blob.
Re-written on every successful batch. The file is small enough
(~50 KB for 5k entries) that rewriting each time is fine.

### Migration from Phase 2

On app startup, if `data/favicons/<uid>/index.json` exists:

1. Load + parse the v2 `IndexFile`.
2. Generate a new salt.
3. For each `Found` entry, read the plaintext `<hostname>.png`, write the
   encrypted `.bin` file, then delete the plaintext file.
4. Write encrypted `index.bin`, delete `index.json`.
5. Mark the user cache as migrated.

The migration happens once per user; log per-user progress at info level.
If it fails midway (e.g. disk full), we leave the plaintext files in place
and fall back to Phase 2 behavior — next launch retries.

### Lifecycle

- `evict_user` unchanged — `fs::remove_dir_all(dir)` wipes the salt,
  `.bin` files, and `index.bin` together. No new surface.

## Verification checklist

- [ ] `ls data/favicons/<uid>/` shows only `.salt`, `index.bin`, and
      opaque-named `.bin` files. No hostnames visible.
- [ ] Copy `data/favicons/<uid>/index.bin` to a separate machine; it
      doesn't decode as JSON without the `.salt` file.
- [ ] Restart; icons render identically.
- [ ] Delete `.salt`; next fetch generates a fresh salt, old files become
      unreadable; service treats them as non-existent and re-fetches. No
      crash.
- [ ] Migration from v2: plaintext files disappear, encrypted files
      appear, icons render identically.
- [ ] `cargo audit` clean — verify `aes-gcm`, `hkdf`, `blake3` have no
      RUSTSECs.

## What this phase explicitly does not do

- No user-provided passphrase — the salt is random per-install. If the
  disk is stolen with the user still logged in, the salt is readable.
  This is consistent with how the SDK keystore behaves.
- No key rotation. Changing the salt invalidates the cache; we accept
  that (it's self-healing).
- No authenticated index header beyond AES-GCM's built-in tag. If someone
  corrupts the file, decryption fails and we re-fetch. We don't try to
  distinguish "malicious corruption" from "disk error".
