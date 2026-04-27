use crate::{auth::Auth, graph};
use anyhow::Result;
use serde_json::json;

pub async fn run(id: &str, json_out: bool) -> Result<()> {
    let auth = Auth::load().await?;
    let path = format!("/users/{}/messages/{}", auth.user_email, id);
    let v = graph::get(
        &auth,
        &path,
        &[("$select", "subject,from,toRecipients,ccRecipients,receivedDateTime,body")],
    )
    .await?;
    let subject = v.get("subject").and_then(|x| x.as_str()).unwrap_or("");
    let from = v
        .pointer("/from/emailAddress/address")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let date = v.get("receivedDateTime").and_then(|x| x.as_str()).unwrap_or("");
    let to: Vec<String> = v
        .get("toRecipients")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| r.pointer("/emailAddress/address").and_then(|x| x.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let cc: Vec<String> = v
        .get("ccRecipients")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| r.pointer("/emailAddress/address").and_then(|x| x.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let raw_body = v.pointer("/body/content").and_then(|x| x.as_str()).unwrap_or("");
    let content_type = v.pointer("/body/contentType").and_then(|x| x.as_str()).unwrap_or("text");
    let body = if content_type.eq_ignore_ascii_case("html") {
        let text = html2text::from_read(raw_body.as_bytes(), 100);
        normalize(&text)
    } else {
        normalize(raw_body)
    };

    if json_out {
        let out = json!({
            "subject": subject,
            "from": from,
            "to": to,
            "cc": cc,
            "date": date,
            "body": body,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Subject: {subject}");
        println!("From:    {from}");
        if !to.is_empty() {
            println!("To:      {}", to.join(", "));
        }
        if !cc.is_empty() {
            println!("Cc:      {}", cc.join(", "));
        }
        println!("Date:    {date}");
        println!();
        println!("{body}");
    }
    Ok(())
}

fn normalize(s: &str) -> String {
    let mut prev_blank = false;
    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if prev_blank {
                continue;
            }
            prev_blank = true;
            out.push('\n');
        } else {
            prev_blank = false;
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out
}
