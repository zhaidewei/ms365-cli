use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Account namespace for Keychain entries (matches the `secret` wrapper
/// at https://gist.github.com/zhaidewei/secret so existing `secret add`
/// entries are read unchanged).
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "agent-secrets";

/// Path to the macOS built-in `security` CLI. Hard-coded (vs PATH lookup)
/// because the silent-Keychain-access trick depends on the caller being
/// the Apple-signed system binary, not some shadowed wrapper on PATH.
#[cfg(target_os = "macos")]
const SECURITY_BIN: &str = "/usr/bin/security";

const TOKEN_ENDPOINT: &str = "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token";
const CACHE_FILE: &str = "molk/token.json";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Serialize, Deserialize)]
struct CachedToken {
    access_token: String,
    expires_at: u64,
}

pub struct Auth {
    pub user_email: String,
    access_token: String,
}

impl Auth {
    pub async fn load() -> Result<Self> {
        let user_email = lookup("molk-prod-user-email", "MOLK_USER_EMAIL")?;
        if let Some(t) = read_cache()? {
            if t.expires_at > now() + 60 {
                return Ok(Self { user_email, access_token: t.access_token });
            }
        }
        let tenant = lookup("molk-prod-tenant-id", "MOLK_TENANT_ID")?;
        let client_id = lookup("molk-prod-client-id", "MOLK_CLIENT_ID")?;
        let client_secret = lookup("molk-prod-client-secret", "MOLK_CLIENT_SECRET")?;
        let url = TOKEN_ENDPOINT.replace("{tenant}", &tenant);
        let resp: TokenResponse = reqwest::Client::new()
            .post(&url)
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("scope", "https://graph.microsoft.com/.default"),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
            .context("token endpoint request failed")?
            .error_for_status()
            .context("token endpoint returned error status")?
            .json()
            .await
            .context("token response not valid JSON")?;
        let cached = CachedToken {
            access_token: resp.access_token.clone(),
            expires_at: now() + resp.expires_in,
        };
        write_cache(&cached)?;
        Ok(Self { user_email, access_token: resp.access_token })
    }

    pub fn bearer(&self) -> &str {
        &self.access_token
    }
}

/// Look up a credential. Tries env var first (portable), then macOS Keychain
/// (account namespace `agent-secrets`, matching the `secret` wrapper script).
fn lookup(secret_name: &str, env_name: &str) -> Result<String> {
    if let Ok(v) = std::env::var(env_name) {
        if !v.trim().is_empty() {
            return Ok(v.trim().to_string());
        }
    }
    if let Some(v) = keychain_lookup(secret_name) {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    Err(anyhow!(
        "missing credential — set env var `{env_name}` or store in macOS Keychain \
         (service `{secret_name}`, account `agent-secrets`)"
    ))
}

/// Read a Keychain entry by shelling out to Apple's `/usr/bin/security`.
///
/// Why shell-out, not the `security-framework` Rust crate? Because Keychain
/// access control is gated on the *calling binary's code signature*. Apple's
/// `/usr/bin/security` is system-signed and pre-trusted by every entry's
/// default partition list, so it reads silently. An unsigned `cargo build`
/// binary calling the API directly would trigger a Keychain authorization
/// prompt on every run — bad UX for distribution. Cost: ~50ms per spawn,
/// hidden behind 1h token cache.
#[cfg(target_os = "macos")]
fn keychain_lookup(secret_name: &str) -> Option<String> {
    let out = std::process::Command::new(SECURITY_BIN)
        .arg("find-generic-password")
        .arg("-a")
        .arg(KEYCHAIN_ACCOUNT)
        .arg("-s")
        .arg(secret_name)
        .arg("-w")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn keychain_lookup(_secret_name: &str) -> Option<String> {
    None
}

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| d.join(CACHE_FILE))
}

fn read_cache() -> Result<Option<CachedToken>> {
    let Some(p) = cache_path() else { return Ok(None) };
    if !p.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&s).ok())
}

fn write_cache(t: &CachedToken) -> Result<()> {
    let Some(p) = cache_path() else { return Ok(()) };
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, serde_json::to_string(t)?)?;
    Ok(())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
