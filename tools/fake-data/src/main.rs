//! One-shot generator that creates real Bitwarden SDK encrypted data and dumps it to
//! `assets/mock-vault.json`. The desktop app embeds the resulting JSON via `include_bytes!`
//! and exercises the SDK's encrypt → store → unlock → decrypt round-trip end-to-end.
//!
//! Run with `cargo run --bin fake-data`.

use std::{
    collections::HashMap,
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bitwarden_core::{
    ClientSettings, UserId,
    key_management::{
        LocalUserDataKeyState, MasterPasswordUnlockData, UserKeyState,
        account_cryptographic_state::WrappedAccountCryptographicState,
        crypto::{InitUserCryptoMethod, InitUserCryptoRequest},
    },
};
use bitwarden_pm::PasswordManagerClient;
use bitwarden_state::repository::{Repository, RepositoryError, RepositoryItem};
use bitwarden_vault::{
    CardView, Cipher, CipherRepromptType, CipherType, CipherView, Folder, FolderView,
    IdentityView, LoginUriView, LoginView, SshKeyView, UriMatchType,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ── Mock vault file schema ─────────────────────────────────────────────────
//
// Mirrored in `crates/desktop/src/sdk.rs`. Keep field names + types in sync.

#[derive(Serialize, Deserialize, Debug)]
struct MockVaultFile {
    users: Vec<MockUser>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MockUser {
    user_id: String,
    email: String,
    display_name: String,
    server_url: String,
    /// Hardcoded master password for dev-only unlock. Never ship this in a real client.
    master_password_dev_only: String,
    unlock_methods: UnlockMethodsCfg,
    kdf: bitwarden_crypto::Kdf,
    encrypted_user_key: bitwarden_crypto::EncString,
    private_key: bitwarden_crypto::EncString,
    ciphers: Vec<Cipher>,
    folders: Vec<Folder>,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnlockMethodsCfg {
    master_password: bool,
    pin: bool,
    biometrics: bool,
}

// ── Hardcoded user specs ───────────────────────────────────────────────────

struct UserSpec {
    user_id: &'static str,
    email: &'static str,
    display_name: &'static str,
    server_url: &'static str,
    password: &'static str,
    unlock_methods: UnlockMethodsCfg,
    ciphers: fn() -> Vec<CipherView>,
    folders: fn() -> Vec<FolderView>,
}

// Stable per-user UUIDs. The SDK's `PasswordManagerClient` expects a single
// `UserId` per client — one client, one user, for the client's entire lifetime
// — so the ID must survive both regenerations of the mock vault and repeated
// unlock calls on the desktop side. We hardcode explicit v4 UUIDs here so
// regenerating `mock-vault.json` produces identical IDs and the desktop app's
// per-user state (HashMap keyed by user_id) stays consistent across builds.
const USER_SPECS: &[UserSpec] = &[
    UserSpec {
        user_id: "11111111-1111-4111-a111-111111111111",
        email: "alice@example.com",
        display_name: "Alice Johnson",
        server_url: "bitwarden.com",
        password: "password",
        unlock_methods: UnlockMethodsCfg {
            master_password: true,
            pin: false,
            biometrics: true,
        },
        ciphers: personal_ciphers,
        folders: personal_folders,
    },
    UserSpec {
        user_id: "22222222-2222-4222-a222-222222222222",
        email: "alice@acmecorp.com",
        display_name: "Alice (Work)",
        server_url: "vault.acmecorp.com",
        password: "123456",
        unlock_methods: UnlockMethodsCfg {
            master_password: true,
            pin: true,
            biometrics: false,
        },
        ciphers: work_ciphers,
        folders: work_folders,
    },
    // Load-test account: ~20k ciphers mixing logins/cards/notes/identities/ssh
    // keys with deterministic pseudo-random names. Exercises layout, scroll,
    // decrypt-list, and filter perf against a realistic-size vault. Unlock
    // password is "loadtest".
    UserSpec {
        user_id: "33333333-3333-4333-a333-333333333333",
        email: "loadtest@example.com",
        display_name: "Load Test",
        server_url: "bitwarden.com",
        password: "loadtest",
        unlock_methods: UnlockMethodsCfg {
            master_password: true,
            pin: false,
            biometrics: false,
        },
        ciphers: load_test_ciphers,
        folders: load_test_folders,
    },
];

// ── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut users = Vec::with_capacity(USER_SPECS.len());

    for spec in USER_SPECS {
        users.push(build_user(spec).await?);
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("mock-vault.json");

    // Compact output: with 20k+ encrypted ciphers, pretty-printing roughly
    // doubles the file size for no practical debugging benefit (no one is
    // reading a 20k-item diff anyway). The desktop crate includes this file
    // via `include_bytes!`, so every byte lands in the final binary.
    let file = std::fs::File::create(&path)?;
    serde_json::to_writer(file, &MockVaultFile { users })?;

    println!(
        "Wrote {} users to {}",
        USER_SPECS.len(),
        path.canonicalize()?.display()
    );
    Ok(())
}

async fn build_user(spec: &UserSpec) -> Result<MockUser, Box<dyn Error>> {
    let client = PasswordManagerClient::new(Some(ClientSettings {
        identity_url: "http://localhost:8080/identity".to_string(),
        api_url: "http://localhost:8080/api".to_string(),
        ..Default::default()
    }));

    // The crypto initializer touches `UserKeyState` and `LocalUserDataKeyState`,
    // and encryption needs the cipher/folder repos for the vault client.
    register_empty_repo::<UserKeyState>(&client);
    register_empty_repo::<LocalUserDataKeyState>(&client);
    register_empty_repo::<Cipher>(&client);
    register_empty_repo::<Folder>(&client);

    let kdf = bitwarden_crypto::Kdf::default_pbkdf2();

    // Stable per-user UUID: parsed from the spec so every regeneration of the
    // mock vault produces the same SDK UserId. One Client = one user, and that
    // UserId must match across `initialize_user_crypto` and every subsequent
    // SDK call on this Client.
    let sdk_user_id = UserId::new(
        uuid::Uuid::parse_str(spec.user_id).expect("USER_SPECS user_id must be a valid UUID"),
    );

    let reg = client.0.auth().make_register_keys(
        spec.email.to_string(),
        spec.password.to_string(),
        kdf.clone(),
    )?;

    client
        .crypto()
        .initialize_user_crypto(InitUserCryptoRequest {
            user_id: Some(sdk_user_id),
            kdf_params: kdf.clone(),
            email: spec.email.to_string(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 {
                private_key: reg.keys.private.clone(),
            },
            method: InitUserCryptoMethod::MasterPasswordUnlock {
                password: spec.password.to_string(),
                master_password_unlock: MasterPasswordUnlockData {
                    kdf: kdf.clone(),
                    master_key_wrapped_user_key: reg.encrypted_user_key.clone(),
                    salt: spec.email.to_string(),
                },
            },
            upgrade_token: None,
        })
        .await?;

    let cipher_views = (spec.ciphers)();
    let total = cipher_views.len();
    if total > 1_000 {
        println!("  encrypting {} ciphers for {}…", total, spec.email);
    }
    let mut ciphers = Vec::with_capacity(total);
    for (i, view) in cipher_views.into_iter().enumerate() {
        let ctx = client.vault().ciphers().encrypt(view)?;
        ciphers.push(ctx.cipher);
        if total > 1_000 && (i + 1) % 5_000 == 0 {
            println!("    {} / {}", i + 1, total);
        }
    }

    let folder_views = (spec.folders)();
    let mut folders = Vec::with_capacity(folder_views.len());
    for view in folder_views {
        folders.push(client.vault().folders().encrypt(view)?);
    }

    Ok(MockUser {
        user_id: spec.user_id.to_string(),
        email: spec.email.to_string(),
        display_name: spec.display_name.to_string(),
        server_url: spec.server_url.to_string(),
        master_password_dev_only: spec.password.to_string(),
        unlock_methods: UnlockMethodsCfg {
            master_password: spec.unlock_methods.master_password,
            pin: spec.unlock_methods.pin,
            biometrics: spec.unlock_methods.biometrics,
        },
        kdf,
        encrypted_user_key: reg.encrypted_user_key,
        private_key: reg.keys.private,
        ciphers,
        folders,
    })
}

fn register_empty_repo<T: RepositoryItem + Clone>(client: &PasswordManagerClient) {
    let repo = Arc::new(MemoryRepo::<T> {
        data: Mutex::new(HashMap::new()),
    });
    client.platform().state().register_client_managed(repo);
}

// ── Cipher view builders ───────────────────────────────────────────────────

fn login(name: &str, username: Option<&str>, uri: Option<&str>) -> CipherView {
    cipher_with(
        name,
        None,
        CipherKind::Login(Box::new(LoginView {
            username: username.map(str::to_string),
            password: Some("fake-password-123".to_string()),
            password_revision_date: None,
            uris: uri.map(|u| {
                vec![LoginUriView {
                    uri: Some(u.to_string()),
                    r#match: Some(UriMatchType::Domain),
                    uri_checksum: None,
                }]
            }),
            totp: None,
            autofill_on_page_load: None,
            fido2_credentials: None,
        })),
    )
}

/// Build a `CipherView` of the given type, with cipher-level metadata pre-filled.
/// The caller plugs in the type-specific view via the `kind` closure.
fn cipher_with(name: &str, notes: Option<String>, kind: CipherKind) -> CipherView {
    let now = Utc::now();
    let (r#type, login, card, identity, secure_note, ssh_key) = match kind {
        CipherKind::Login(l) => (CipherType::Login, Some(*l), None, None, None, None),
        CipherKind::Card(c) => (CipherType::Card, None, Some(*c), None, None, None),
        CipherKind::Identity(i) => (CipherType::Identity, None, None, Some(*i), None, None),
        CipherKind::SecureNote => (
            CipherType::SecureNote,
            None,
            None,
            None,
            Some(bitwarden_vault::SecureNoteView {
                r#type: bitwarden_vault::SecureNoteType::Generic,
            }),
            None,
        ),
        CipherKind::SshKey(k) => (CipherType::SshKey, None, None, None, None, Some(*k)),
    };
    CipherView {
        id: Some(bitwarden_vault::CipherId::new(uuid::Uuid::new_v4())),
        organization_id: None,
        folder_id: None,
        collection_ids: vec![],
        key: None,
        name: name.to_string(),
        notes,
        r#type,
        login,
        identity,
        card,
        secure_note,
        ssh_key,
        favorite: false,
        reprompt: CipherRepromptType::None,
        organization_use_totp: false,
        edit: true,
        permissions: None,
        view_password: true,
        local_data: None,
        attachments: None,
        attachment_decryption_failures: None,
        fields: None,
        password_history: None,
        creation_date: now,
        deleted_date: None,
        revision_date: now,
        archived_date: None,
    }
}

enum CipherKind {
    Login(Box<LoginView>),
    Card(Box<CardView>),
    Identity(Box<IdentityView>),
    SecureNote,
    SshKey(Box<SshKeyView>),
}

fn note(name: &str) -> CipherView {
    cipher_with(name, Some(format!("Notes for {name}")), CipherKind::SecureNote)
}

fn card(
    name: &str,
    cardholder: &str,
    brand: &str,
    number: &str,
    exp_month: &str,
    exp_year: &str,
    code: &str,
) -> CipherView {
    cipher_with(
        name,
        None,
        CipherKind::Card(Box::new(CardView {
            cardholder_name: Some(cardholder.to_string()),
            exp_month: Some(exp_month.to_string()),
            exp_year: Some(exp_year.to_string()),
            code: Some(code.to_string()),
            brand: Some(brand.to_string()),
            number: Some(number.to_string()),
        })),
    )
}

fn identity(
    name: &str,
    title: &str,
    first: &str,
    last: &str,
    email: &str,
    phone: &str,
) -> CipherView {
    cipher_with(
        name,
        None,
        CipherKind::Identity(Box::new(IdentityView {
            title: Some(title.to_string()),
            first_name: Some(first.to_string()),
            middle_name: None,
            last_name: Some(last.to_string()),
            address1: Some("742 Evergreen Terrace".to_string()),
            address2: None,
            address3: None,
            city: Some("Springfield".to_string()),
            state: Some("IL".to_string()),
            postal_code: Some("62704".to_string()),
            country: Some("USA".to_string()),
            company: None,
            email: Some(email.to_string()),
            phone: Some(phone.to_string()),
            ssn: None,
            username: None,
            passport_number: None,
            license_number: None,
        })),
    )
}

fn ssh_key(name: &str, public_key: &str, private_key: &str, fingerprint: &str) -> CipherView {
    cipher_with(
        name,
        None,
        CipherKind::SshKey(Box::new(SshKeyView {
            private_key: private_key.to_string(),
            public_key: public_key.to_string(),
            fingerprint: fingerprint.to_string(),
        })),
    )
}

fn folder(name: &str) -> FolderView {
    FolderView {
        id: Some(bitwarden_vault::FolderId::new(uuid::Uuid::new_v4())),
        name: name.to_string(),
        revision_date: Utc::now(),
    }
}

fn personal_ciphers() -> Vec<CipherView> {
    vec![
        login("Gmail", Some("alice@example.com"), Some("mail.google.com")),
        login("GitHub", Some("alice-dev"), Some("github.com")),
        login("Netflix", Some("alice@example.com"), Some("netflix.com")),
        login("Amazon", Some("alice@example.com"), Some("amazon.com")),
        login("Reddit", Some("alice_online"), Some("reddit.com")),
        login("Steam", Some("alice_gamer"), Some("store.steampowered.com")),
        login("Spotify", Some("alice@example.com"), Some("spotify.com")),
        login(
            "Bank of Example",
            Some("alice.johnson"),
            Some("bankofexample.com"),
        ),
        login("Discord", Some("alice#1234"), Some("discord.com")),
        login("Twitter / X", Some("@alice_j"), Some("x.com")),
        login("LinkedIn", Some("alice@example.com"), Some("linkedin.com")),
        login("Dropbox", Some("alice@example.com"), Some("dropbox.com")),
        card(
            "Personal Visa",
            "Alice Johnson",
            "Visa",
            "4111 1111 1111 1111",
            "08",
            "2029",
            "123",
        ),
        card(
            "Travel Mastercard",
            "Alice Johnson",
            "Mastercard",
            "5555 5555 5555 4444",
            "12",
            "2027",
            "456",
        ),
        identity(
            "Alice Johnson",
            "Ms",
            "Alice",
            "Johnson",
            "alice@example.com",
            "+1 555 0100",
        ),
        note("Recovery Codes Backup"),
        note("WiFi Passwords"),
        ssh_key(
            "GitHub SSH Key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleAlicePublicKey alice@desktop",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nfake-private-key-bytes\n-----END OPENSSH PRIVATE KEY-----\n",
            "SHA256:abc123ExampleFingerprintAliceDesktop",
        ),
    ]
}

fn personal_folders() -> Vec<FolderView> {
    vec![folder("Personal"), folder("Finance")]
}

fn work_ciphers() -> Vec<CipherView> {
    vec![
        login(
            "Company Jira",
            Some("ajohnson@acmecorp.com"),
            Some("acmecorp.atlassian.net"),
        ),
        login("Company GitHub", Some("alice-acme"), Some("github.com")),
        login(
            "AWS Console",
            Some("ajohnson@acmecorp.com"),
            Some("aws.amazon.com"),
        ),
        login("Slack", Some("ajohnson"), Some("acmecorp.slack.com")),
        card(
            "Corporate Card",
            "Acme Corp",
            "Amex",
            "3782 822463 10005",
            "06",
            "2028",
            "7890",
        ),
        note("Production DB Credentials"),
        ssh_key(
            "Deploy SSH Key",
            "ssh-rsa AAAAB3NzaC1yc2EExampleDeployKey deploy@acmecorp",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nfake-deploy-private-bytes\n-----END OPENSSH PRIVATE KEY-----\n",
            "SHA256:def456ExampleFingerprintDeploy",
        ),
    ]
}

fn work_folders() -> Vec<FolderView> {
    vec![folder("Work"), folder("Infrastructure")]
}

// ── Load-test generators ───────────────────────────────────────────────────
//
// Produces ~20k ciphers with deterministic pseudo-random names so the JSON
// output is stable across regenerations. The mix (≈80/10/5/3/2) approximates
// what a real heavy user would have in their vault.

/// Total cipher count for the load-test account. Tune here if 20k turns out
/// to be too large (or too small).
const LOAD_TEST_CIPHER_COUNT: usize = 20_000;

/// Fixed seed for the load-test PRNG. Changing this reshuffles all generated
/// names/usernames, so only touch it if you deliberately want to invalidate
/// the checked-in mock vault.
const LOAD_TEST_SEED: u64 = 0xC0FFEE_DEADBEEF;

/// Minimal xorshift64 PRNG. Deterministic, no dependencies, good enough for
/// picking words out of a bank — we are not generating crypto randomness here.
struct Prng {
    state: u64,
}

impl Prng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = (self.next_u64() as usize) % slice.len();
        &slice[idx]
    }
    fn range(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }
}

const LOAD_TEST_COMPANIES: &[&str] = &[
    "Acme", "Globex", "Initech", "Umbrella", "Wayne", "Stark", "Wonka", "Tyrell", "Cyberdyne",
    "Hooli", "Pied Piper", "Soylent", "Dunder Mifflin", "Vandelay", "Massive Dynamic", "Oscorp",
    "LexCorp", "Aperture", "Black Mesa", "Rekall", "Weyland", "Nakatomi", "InGen", "Parabellum",
    "Genco", "Virtucon", "Buy n Large", "Omni Consumer", "Prestige Worldwide", "Scruffy",
];

const LOAD_TEST_KINDS: &[&str] = &[
    "Portal",
    "Dashboard",
    "Admin",
    "VPN",
    "Support",
    "Intranet",
    "Cloud",
    "Staging",
    "Prod",
    "CI",
    "Git",
    "Wiki",
    "Billing",
    "Analytics",
    "Monitoring",
    "Metrics",
    "Reports",
    "Ops",
    "Mail",
    "Chat",
    "Forum",
    "Bug Tracker",
    "CDN",
    "DNS",
    "Registrar",
    "Backup",
    "Archive",
    "API",
    "Console",
    "Hub",
];

const LOAD_TEST_USERNAMES: &[&str] = &[
    "alice.johnson",
    "ajohnson",
    "alice_j",
    "a.johnson",
    "alice@loadtest.dev",
    "loadtest-admin",
    "service-account",
    "alice.j@example.com",
    "alice2024",
];

const LOAD_TEST_TLDS: &[&str] = &[
    "com", "io", "dev", "net", "org", "co", "app", "cloud", "tech", "systems",
];

const LOAD_TEST_CARD_BRANDS: &[&str] = &["Visa", "Mastercard", "Amex", "Discover"];

const LOAD_TEST_FIRST_NAMES: &[&str] = &[
    "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Heidi", "Ivan", "Judy", "Mallory",
    "Oscar", "Peggy", "Trent", "Victor", "Walter",
];

const LOAD_TEST_LAST_NAMES: &[&str] = &[
    "Smith",
    "Jones",
    "Brown",
    "Taylor",
    "Wilson",
    "Davies",
    "Evans",
    "Thomas",
    "Roberts",
    "Walker",
    "Wright",
    "Robinson",
];

fn load_test_ciphers() -> Vec<CipherView> {
    let mut prng = Prng::new(LOAD_TEST_SEED);
    let mut out: Vec<CipherView> = Vec::with_capacity(LOAD_TEST_CIPHER_COUNT);

    // Type distribution: 80% logins, 10% notes, 5% cards, 3% identities, 2% ssh keys.
    // We derive each cipher's type from its index rather than rolling PRNG for
    // the distribution, so tuning LOAD_TEST_CIPHER_COUNT keeps the ratios stable.
    for i in 0..LOAD_TEST_CIPHER_COUNT {
        let bucket = i % 100;
        let cipher = if bucket < 80 {
            // Login
            let company = prng.pick(LOAD_TEST_COMPANIES);
            let kind = prng.pick(LOAD_TEST_KINDS);
            let tld = prng.pick(LOAD_TEST_TLDS);
            let username = prng.pick(LOAD_TEST_USERNAMES);
            let name = format!("{company} {kind} #{i}");
            let slug = company.to_ascii_lowercase().replace(' ', "-");
            let uri = format!("{slug}.{tld}");
            login(&name, Some(username), Some(&uri))
        } else if bucket < 90 {
            // Secure note
            let company = prng.pick(LOAD_TEST_COMPANIES);
            note(&format!("{company} runbook #{i}"))
        } else if bucket < 95 {
            // Card
            let brand = prng.pick(LOAD_TEST_CARD_BRANDS);
            let first = prng.pick(LOAD_TEST_FIRST_NAMES);
            let last = prng.pick(LOAD_TEST_LAST_NAMES);
            let cardholder = format!("{first} {last}");
            let number = format!(
                "{:04} {:04} {:04} {:04}",
                prng.range(10000),
                prng.range(10000),
                prng.range(10000),
                prng.range(10000)
            );
            let exp_month = format!("{:02}", 1 + prng.range(12));
            let exp_year = format!("{}", 2026 + prng.range(6));
            let code = format!("{:03}", prng.range(1000));
            card(
                &format!("{brand} #{i}"),
                &cardholder,
                brand,
                &number,
                &exp_month,
                &exp_year,
                &code,
            )
        } else if bucket < 98 {
            // Identity
            let first = prng.pick(LOAD_TEST_FIRST_NAMES);
            let last = prng.pick(LOAD_TEST_LAST_NAMES);
            let first_lower = first.to_ascii_lowercase();
            let last_lower = last.to_ascii_lowercase();
            let name = format!("{first} {last} #{i}");
            let email = format!("{first_lower}.{last_lower}@example.com");
            let phone = format!(
                "+1 555 {:04}",
                prng.range(10000)
            );
            identity(&name, "Ms", first, last, &email, &phone)
        } else {
            // SSH key
            let company = prng.pick(LOAD_TEST_COMPANIES);
            let slug = company.to_ascii_lowercase().replace(' ', "-");
            let name = format!("{company} Deploy Key #{i}");
            let public_key = format!(
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI{slug}FakePublicKey{i} deploy@{slug}"
            );
            let private_key = format!(
                "-----BEGIN OPENSSH PRIVATE KEY-----\nfake-private-key-{slug}-{i}\n-----END OPENSSH PRIVATE KEY-----\n"
            );
            let fingerprint = format!("SHA256:loadtest-{slug}-{i:08x}");
            ssh_key(&name, &public_key, &private_key, &fingerprint)
        };
        out.push(cipher);
    }

    out
}

fn load_test_folders() -> Vec<FolderView> {
    vec![
        folder("Personal"),
        folder("Work"),
        folder("Infrastructure"),
        folder("Archived"),
        folder("One-Time"),
    ]
}

// ── In-memory repository ───────────────────────────────────────────────────

struct MemoryRepo<T: RepositoryItem + Clone> {
    data: Mutex<HashMap<String, T>>,
}

#[async_trait::async_trait]
impl<T: RepositoryItem + Clone> Repository<T> for MemoryRepo<T> {
    async fn get(&self, key: T::Key) -> Result<Option<T>, RepositoryError> {
        Ok(self.data.lock().unwrap().get(&key.to_string()).cloned())
    }
    async fn list(&self) -> Result<Vec<T>, RepositoryError> {
        Ok(self.data.lock().unwrap().values().cloned().collect())
    }
    async fn set(&self, key: T::Key, value: T) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().insert(key.to_string(), value);
        Ok(())
    }
    async fn set_bulk(&self, values: Vec<(T::Key, T)>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for (k, v) in values {
            map.insert(k.to_string(), v);
        }
        Ok(())
    }
    async fn remove(&self, key: T::Key) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().remove(&key.to_string());
        Ok(())
    }
    async fn remove_bulk(&self, keys: Vec<T::Key>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for k in keys {
            map.remove(&k.to_string());
        }
        Ok(())
    }
    async fn remove_all(&self) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().clear();
        Ok(())
    }
}
