use crate::{auth::Auth, graph};
use anyhow::Result;
use serde_json::json;

pub async fn run(query: &str, count: u32) -> Result<()> {
    let auth = Auth::load().await?;
    let path = format!("/users/{}/messages", auth.user_email);
    let q = format!("\"{query}\"");
    let top = count.to_string();
    let v = graph::get(
        &auth,
        &path,
        &[
            ("$search", &q),
            ("$top", &top),
            ("$select", "id,subject,from,receivedDateTime,bodyPreview"),
        ],
    )
    .await?;
    let empty = vec![];
    let items = v.get("value").and_then(|x| x.as_array()).unwrap_or(&empty);
    for e in items {
        let preview = e
            .get("bodyPreview")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let preview = if preview.chars().count() > 200 {
            preview.chars().take(200).collect::<String>()
        } else {
            preview.to_string()
        };
        let line = json!({
            "id": e.get("id").and_then(|x| x.as_str()).unwrap_or(""),
            "from": e.pointer("/from/emailAddress/address").and_then(|x| x.as_str()).unwrap_or(""),
            "subject": e.get("subject").and_then(|x| x.as_str()).unwrap_or(""),
            "date": e.get("receivedDateTime").and_then(|x| x.as_str()).unwrap_or(""),
            "preview": preview,
        });
        println!("{}", serde_json::to_string(&line)?);
    }
    Ok(())
}
