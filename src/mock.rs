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

fn mock_vault_items_personal() -> Vec<CipherItem> {
    vec![
        CipherItem {
            name: "Gmail".into(),
            username: Some("alice@example.com".into()),
            url: Some("mail.google.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "GitHub".into(),
            username: Some("alice-dev".into()),
            url: Some("github.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Netflix".into(),
            username: Some("alice@example.com".into()),
            url: Some("netflix.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Amazon".into(),
            username: Some("alice@example.com".into()),
            url: Some("amazon.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Reddit".into(),
            username: Some("alice_online".into()),
            url: Some("reddit.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Steam".into(),
            username: Some("alice_gamer".into()),
            url: Some("store.steampowered.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Spotify".into(),
            username: Some("alice@example.com".into()),
            url: Some("spotify.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Bank of Example".into(),
            username: Some("alice.johnson".into()),
            url: Some("bankofexample.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Home WiFi Router".into(),
            username: Some("admin".into()),
            url: Some("192.168.1.1".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Discord".into(),
            username: Some("alice#1234".into()),
            url: Some("discord.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Twitter / X".into(),
            username: Some("@alice_j".into()),
            url: Some("x.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "LinkedIn".into(),
            username: Some("alice@example.com".into()),
            url: Some("linkedin.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Dropbox".into(),
            username: Some("alice@example.com".into()),
            url: Some("dropbox.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Personal Visa".into(),
            username: None,
            url: None,
            category: CipherCategory::Card,
        },
        CipherItem {
            name: "Debit Card".into(),
            username: None,
            url: None,
            category: CipherCategory::Card,
        },
        CipherItem {
            name: "Alice Johnson".into(),
            username: Some("Personal identity".into()),
            url: None,
            category: CipherCategory::Identity,
        },
        CipherItem {
            name: "Recovery Codes Backup".into(),
            username: None,
            url: None,
            category: CipherCategory::SecureNote,
        },
        CipherItem {
            name: "WiFi Passwords".into(),
            username: None,
            url: None,
            category: CipherCategory::SecureNote,
        },
        CipherItem {
            name: "GitHub SSH Key".into(),
            username: Some("git@github.com".into()),
            url: None,
            category: CipherCategory::SshKey,
        },
    ]
}

fn mock_vault_items_work() -> Vec<CipherItem> {
    vec![
        CipherItem {
            name: "Company Jira".into(),
            username: Some("ajohnson@acmecorp.com".into()),
            url: Some("acmecorp.atlassian.net".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Company GitHub".into(),
            username: Some("alice-acme".into()),
            url: Some("github.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "AWS Console".into(),
            username: Some("ajohnson@acmecorp.com".into()),
            url: Some("aws.amazon.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Slack".into(),
            username: Some("ajohnson".into()),
            url: Some("acmecorp.slack.com".into()),
            category: CipherCategory::Login,
        },
        CipherItem {
            name: "Corporate Card".into(),
            username: None,
            url: None,
            category: CipherCategory::Card,
        },
        CipherItem {
            name: "Production DB Credentials".into(),
            username: None,
            url: None,
            category: CipherCategory::SecureNote,
        },
        CipherItem {
            name: "Deploy SSH Key".into(),
            username: Some("deploy@acmecorp.com".into()),
            url: None,
            category: CipherCategory::SshKey,
        },
    ]
}
