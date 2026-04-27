# ms365-cli

Token-thrifty Outlook CLI for LLM agents — read your Microsoft 365 mailbox via Graph API without burning context on HTML noise.

Built because Outlook MCP servers transport email bodies as raw HTML (~85% CSS noise from `MsoNormal` classes), which wastes thousands of tokens per email when an LLM agent like Claude Code reads them. `ms365-cli` strips HTML to plain text, trims fields, and skips the schema overhead of an MCP server.

## Features

- 🪶 **Plain-text output** — html2text strips `MsoNormal` & friends; -85% bytes vs raw HTML body
- 🔐 **Credentials in keychain** — first-class macOS Keychain support via [`secret`](https://gist.github.com/zhaidewei/secret); env-var fallback for CI/Linux
- ⚡ **Token caching** — access token cached to `~/Library/Caches/ms365-cli/` for 1 hour
- 📦 **Single 3.7 MB binary** — vs 50+ MB for a Python MCP equivalent
- 🎯 **Two commands only** — `search` and `read`. No bloat.

## Install

```bash
git clone git@github.com:zhaidewei/ms365-cli.git
cd ms365-cli
cargo build --release
ln -sf "$PWD/target/release/ms365" ~/.local/bin/ms365  # ensure ~/.local/bin in PATH
```

Requires Rust 1.75+.

## Setup

1. **Register an Entra (Azure AD) app** in your Microsoft 365 tenant
   - Single-tenant
   - Application permissions: `Mail.Read` (and `Mail.Send` if you extend later)
   - Grant admin consent
   - Create a client secret
2. **Provide credentials** — either via env vars or macOS Keychain:

   **Env vars (any OS):**
   ```bash
   export MS365_CLIENT_ID=...
   export MS365_TENANT_ID=...
   export MS365_CLIENT_SECRET=...
   export MS365_USER_EMAIL=you@yourdomain.com
   ```

   **macOS Keychain (recommended on Mac, requires the [`secret`](https://gist.github.com/zhaidewei/secret) wrapper):**
   ```bash
   secret add ms365-prod-client-id     "Entra app client_id"
   secret add ms365-prod-tenant-id     "M365 tenant id"
   secret add ms365-prod-client-secret "Entra app client secret"
   secret add ms365-prod-user-email    "Target mailbox UPN"
   ```

   `ms365` tries env vars first, then falls back to Keychain.

## Usage

```bash
# Search by keyword (subject + body). NDJSON one-per-line to stdout.
ms365 search "invoice" -n 10

# Read one message, plain-text body
ms365 read AAMkAD...

# Read with full JSON output (body still HTML-stripped)
ms365 read AAMkAD... --json
```

### Token economy in numbers

For a typical "find an email" workflow (1× search, 3× read):

| Scenario                     | Bytes   |
|------------------------------|---------|
| Outlook MCP (HTML body)      | ~20,800 |
| `ms365` CLI (plain-text)     | ~3,000  |
| **Reduction**                | **−86%** |

Plus you eliminate the ~700 tokens of MCP tool schema that would otherwise live in input context permanently.

## Auth model

Uses Microsoft Graph **Application Permissions** (client credentials flow). The CLI is intended for *your own* mailbox, with a single-tenant Entra app you own. Don't use this against mailboxes you don't administer.

## Limitations

- Read-only (no send / draft / reply yet — by design, scope creep is the enemy)
- Application Permissions only (no delegated user flow)
- Tested only against Microsoft 365 Business / Exchange Online (not personal `@outlook.com` accounts)

## License

MIT — see [LICENSE](LICENSE).
