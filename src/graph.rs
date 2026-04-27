use crate::auth::Auth;
use anyhow::{Context, Result};
use serde_json::Value;

const BASE: &str = "https://graph.microsoft.com/v1.0";

pub async fn get(auth: &Auth, path: &str, query: &[(&str, &str)]) -> Result<Value> {
    let url = format!("{BASE}{path}");
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(auth.bearer())
        .header("ConsistencyLevel", "eventual")
        .query(query)
        .send()
        .await
        .with_context(|| format!("GET {url} failed"))?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("GET {url} -> {status}: {text}");
    }
    Ok(serde_json::from_str(&text).context("graph response not JSON")?)
}
