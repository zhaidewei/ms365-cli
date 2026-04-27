use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_ENDPOINT: &str = "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token";
const CACHE_FILE: &str = "ms365-cli/token.json";

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
        let user_email = lookup("ms365-prod-user-email", "MS365_USER_EMAIL")?;
        if let Some(t) = read_cache()? {
            if t.expires_at > now() + 60 {
                return Ok(Self { user_email, access_token: t.access_token });
            }
        }
        let tenant = lookup("ms365-prod-tenant-id", "MS365_TENANT_ID")?;
        let client_id = lookup("ms365-prod-client-id", "MS365_CLIENT_ID")?;
        let client_secret = lookup("ms365-prod-client-secret", "MS365_CLIENT_SECRET")?;
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

/// Look up a credential. Tries env var first (portable), then macOS Keychain via `secret` CLI.
fn lookup(secret_name: &str, env_name: &str) -> Result<String> {
    if let Ok(v) = std::env::var(env_name) {
        if !v.trim().is_empty() {
            return Ok(v.trim().to_string());
        }
    }
    let out = Command::new("secret").arg("get").arg(secret_name).output();
    match out {
        Ok(o) if o.status.success() => Ok(String::from_utf8(o.stdout)?.trim().to_string()),
        _ => Err(anyhow!(
            "missing credential — set env var `{env_name}` or run `secret add {secret_name}`"
        )),
    }
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
