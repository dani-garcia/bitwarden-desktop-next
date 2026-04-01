use std::collections::HashMap;

use crate::state::{CipherCategory, CipherItem, UserId, UserSession};

pub fn mock_users() -> (HashMap<UserId, UserSession>, UserId) {
    let mut users = HashMap::new();

    let user1_id = "user-1".to_string();
    users.insert(
        user1_id.clone(),
        UserSession {
            email: "alice@example.com".into(),
            display_name: "Alice Johnson".into(),
            server_url: "bitwarden.com".into(),
            locked: true,
            vault_items: mock_vault_items_personal(),
        },
    );

    let user2_id = "user-2".to_string();
    users.insert(
        user2_id,
        UserSession {
            email: "alice@acmecorp.com".into(),
            display_name: "Alice (Work)".into(),
            server_url: "vault.acmecorp.com".into(),
            locked: true,
            vault_items: mock_vault_items_work(),
        },
    );

    (users, user1_id)
}

fn item(
    id: &str,
    name: &str,
    username: Option<&str>,
    url: Option<&str>,
    category: CipherCategory,
) -> CipherItem {
    CipherItem {
        id: id.into(),
        name: name.into(),
        username: username.map(Into::into),
        url: url.map(Into::into),
        category,
    }
}

fn mock_vault_items_personal() -> Vec<CipherItem> {
    use CipherCategory::*;
    vec![
        item(
            "c1a0-0001",
            "Gmail",
            Some("alice@example.com"),
            Some("mail.google.com"),
            Login,
        ),
        item(
            "c1a0-0002",
            "GitHub",
            Some("alice-dev"),
            Some("github.com"),
            Login,
        ),
        item(
            "c1a0-0003",
            "Netflix",
            Some("alice@example.com"),
            Some("netflix.com"),
            Login,
        ),
        item(
            "c1a0-0004",
            "Amazon",
            Some("alice@example.com"),
            Some("amazon.com"),
            Login,
        ),
        item(
            "c1a0-0005",
            "Reddit",
            Some("alice_online"),
            Some("reddit.com"),
            Login,
        ),
        item(
            "c1a0-0006",
            "Steam",
            Some("alice_gamer"),
            Some("store.steampowered.com"),
            Login,
        ),
        item(
            "c1a0-0007",
            "Spotify",
            Some("alice@example.com"),
            Some("spotify.com"),
            Login,
        ),
        item(
            "c1a0-0008",
            "Bank of Example",
            Some("alice.johnson"),
            Some("bankofexample.com"),
            Login,
        ),
        item(
            "c1a0-0009",
            "Home WiFi Router",
            Some("admin"),
            Some("192.168.1.1"),
            Login,
        ),
        item(
            "c1a0-000a",
            "Discord",
            Some("alice#1234"),
            Some("discord.com"),
            Login,
        ),
        item(
            "c1a0-000b",
            "Twitter / X",
            Some("@alice_j"),
            Some("x.com"),
            Login,
        ),
        item(
            "c1a0-000c",
            "LinkedIn",
            Some("alice@example.com"),
            Some("linkedin.com"),
            Login,
        ),
        item(
            "c1a0-000d",
            "Dropbox",
            Some("alice@example.com"),
            Some("dropbox.com"),
            Login,
        ),
        item("c1a0-000e", "Personal Visa", None, None, Card),
        item("c1a0-000f", "Debit Card", None, None, Card),
        item(
            "c1a0-0010",
            "Alice Johnson",
            Some("Personal identity"),
            None,
            Identity,
        ),
        item("c1a0-0011", "Recovery Codes Backup", None, None, SecureNote),
        item("c1a0-0012", "WiFi Passwords", None, None, SecureNote),
        item(
            "c1a0-0013",
            "GitHub SSH Key",
            Some("git@github.com"),
            None,
            SshKey,
        ),
    ]
}

fn mock_vault_items_work() -> Vec<CipherItem> {
    use CipherCategory::*;
    vec![
        item(
            "c2b0-0001",
            "Company Jira",
            Some("ajohnson@acmecorp.com"),
            Some("acmecorp.atlassian.net"),
            Login,
        ),
        item(
            "c2b0-0002",
            "Company GitHub",
            Some("alice-acme"),
            Some("github.com"),
            Login,
        ),
        item(
            "c2b0-0003",
            "AWS Console",
            Some("ajohnson@acmecorp.com"),
            Some("aws.amazon.com"),
            Login,
        ),
        item(
            "c2b0-0004",
            "Slack",
            Some("ajohnson"),
            Some("acmecorp.slack.com"),
            Login,
        ),
        item("c2b0-0005", "Corporate Card", None, None, Card),
        item(
            "c2b0-0006",
            "Production DB Credentials",
            None,
            None,
            SecureNote,
        ),
        item(
            "c2b0-0007",
            "Deploy SSH Key",
            Some("deploy@acmecorp.com"),
            None,
            SshKey,
        ),
    ]
}
