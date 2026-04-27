mod auth;
mod graph;
mod cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "molk", about = "Token-thrifty Outlook mail CLI for LLM agents (Microsoft Graph)", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Search emails by keyword (subject + body). NDJSON to stdout.
    Search {
        query: String,
        #[arg(short = 'n', long, default_value_t = 10)]
        count: u32,
    },
    /// Read one email by id. Defaults to plain-text body.
    Read {
        id: String,
        /// Emit full JSON (body still HTML-stripped) instead of plain text
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Search { query, count } => cmd::search::run(&query, count).await,
        Cmd::Read { id, json } => cmd::read::run(&id, json).await,
    }
}
