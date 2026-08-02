use serde::Deserialize;
use std::net::IpAddr;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hub: HubConfig,
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_database")]
    pub database_path: String,
    /// Canonical externally reachable HTTPS URL. It is never inferred from an
    /// untrusted Host or forwarded header.
    pub public_url: String,
    #[serde(default = "default_dist")]
    pub dist_dir: String,
    /// SHA-256 of the offline Ed25519 release public-key PEM.
    pub release_public_key_sha256: String,
    #[serde(default)]
    pub trusted_proxies: Vec<IpAddr>,
    #[serde(default = "yes")]
    pub secure_cookies: bool,
    #[serde(default = "default_session_hours")]
    pub session_hours: u64,
    #[serde(default = "default_stale")]
    pub stale_after_secs: i64,
    #[serde(default = "default_offline")]
    pub offline_after_secs: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DashboardConfig {
    #[serde(default = "default_title")]
    pub title: String,
    /// Argon2id PHC string produced by `parade-hub hash-password`.
    pub password_hash: String,
}

fn default_listen() -> String {
    "127.0.0.1:8008".into()
}
fn default_database() -> String {
    "/var/lib/parade/parade.sqlite3".into()
}
fn default_dist() -> String {
    "/var/lib/parade-dist".into()
}
fn default_session_hours() -> u64 {
    12
}
fn default_stale() -> i64 {
    600
}
fn default_offline() -> i64 {
    1_800
}
fn default_title() -> String {
    "Parade".into()
}
fn yes() -> bool {
    true
}

pub fn load(path: &str) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read config {path}: {error}"))?;
    let config: Config =
        toml::from_str(&raw).map_err(|error| format!("bad config {path}: {error}"))?;
    if !config.dashboard.password_hash.starts_with("$argon2id$") {
        return Err("dashboard.password_hash must be a valid Argon2id PHC string; pipe the password to `parade-hub hash-password`".into());
    }
    let public = config.hub.public_url.trim_end_matches('/');
    if !public.starts_with("https://") && !is_loopback_url(public) {
        return Err(
            "hub.public_url must use https (http is permitted only for loopback development)"
                .into(),
        );
    }
    let authority = public
        .split_once("://")
        .map(|(_, value)| value)
        .unwrap_or_default();
    if authority.is_empty()
        || authority.contains('/')
        || !authority
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:[]".contains(&byte))
    {
        return Err("hub.public_url must be an origin URL without a path, query, fragment, credentials, or shell metacharacters".into());
    }
    if config.hub.stale_after_secs <= 0
        || config.hub.offline_after_secs <= config.hub.stale_after_secs
    {
        return Err("offline_after_secs must be greater than positive stale_after_secs".into());
    }
    if config.hub.release_public_key_sha256.len() != 64
        || !config
            .hub
            .release_public_key_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(
            "release_public_key_sha256 must pin the 64-character SHA-256 of release-public.pem"
                .into(),
        );
    }
    if !config.hub.secure_cookies && !is_loopback_url(public) {
        return Err(
            "secure_cookies=false is permitted only for exact loopback development URLs".into(),
        );
    }
    Ok(config)
}

fn is_loopback_url(value: &str) -> bool {
    let Some(authority) = value.strip_prefix("http://") else {
        return false;
    };
    if authority.contains('/') || authority.contains('@') {
        return false;
    }
    authority == "localhost"
        || authority.strip_prefix("localhost:").is_some_and(valid_port)
        || authority == "127.0.0.1"
        || authority.strip_prefix("127.0.0.1:").is_some_and(valid_port)
        || authority == "[::1]"
        || authority.strip_prefix("[::1]:").is_some_and(valid_port)
}

fn valid_port(value: &str) -> bool {
    value.parse::<u16>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_or_plaintext_admin_setup_fails_closed() {
        assert!(!is_loopback_url("http://localhost.attacker.example"));
        assert!(!is_loopback_url("http://127.0.0.1.evil"));
        assert!(is_loopback_url("http://localhost:8008"));
        let path =
            std::env::temp_dir().join(format!("parade-config-test-{}.toml", std::process::id()));
        std::fs::write(
            &path,
            r#"
[hub]
public_url = "http://127.0.0.1:8008"
[dashboard]
password_hash = ""
"#,
        )
        .unwrap();
        assert!(load(path.to_str().unwrap()).is_err());
        std::fs::write(
            &path,
            r#"
[hub]
public_url = "http://localhost.attacker.example"
secure_cookies = false
[dashboard]
password_hash = "$argon2id$placeholder"
"#,
        )
        .unwrap();
        assert!(load(path.to_str().unwrap()).is_err());
        std::fs::write(
            &path,
            r#"
[hub]
public_url = "https://example.com/$(unsafe)"
[dashboard]
password_hash = "$argon2id$placeholder"
"#,
        )
        .unwrap();
        assert!(load(path.to_str().unwrap()).is_err());
        let _ = std::fs::remove_file(path);
    }
}
