# ms365-cli

[**English**](README.md) · [中文](README.zh-CN.md)

Token-thrifty Outlook CLI for LLM agents — read your Microsoft 365 mailbox via Graph API without burning context on HTML noise.

Built because Outlook MCP servers transport email bodies as raw HTML (~85% CSS noise from `MsoNormal` classes), which wastes thousands of tokens per email when an LLM agent like Claude Code reads them. `ms365-cli` strips HTML to plain text, trims fields, and skips the schema overhead of an MCP server.

## Features

- 🪶 **Plain-text output** — html2text strips `MsoNormal` & friends; -85% bytes vs raw HTML body
- 🔐 **Credentials in keychain** — direct macOS Keychain access via Apple's Security framework (no shell-out, no runtime deps); env-var fallback for CI/Linux
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

### 1. Register an Entra (Azure AD) app

You need a single-tenant Entra app with Application Permissions on Microsoft Graph. One-time, ~5 minutes.

1. Sign in to [Azure Portal](https://portal.azure.com) → **Microsoft Entra ID** (formerly Azure AD)
2. Left menu → **App registrations** → **New registration**
   - **Name**: `ms365-cli` (anything you like)
   - **Supported account types**: *Accounts in this organizational directory only (Single tenant)*
   - **Redirect URI**: leave empty (server-to-server flow, no callback)
   - Click **Register**
3. On the **Overview** page, copy:
   - **Application (client) ID** → this is your `MS365_CLIENT_ID`
   - **Directory (tenant) ID** → this is your `MS365_TENANT_ID`
4. Left menu → **Certificates & secrets** → **Client secrets** → **+ New client secret**
   - **Description**: `ms365-cli`
   - **Expires**: 24 months recommended (rotate before expiry)
   - Click **Add**
   - **Copy the *Value* immediately** — once the page refreshes you can no longer see it. → this is your `MS365_CLIENT_SECRET`
5. Left menu → **API permissions** → **+ Add a permission**
   - Select **Microsoft Graph** → **Application permissions** (NOT Delegated)
   - Search and tick `Mail.Read`
   - (Optional) `Mail.Send` if you extend the CLI later
   - Click **Add permissions**
6. **Grant admin consent for [your tenant]** ← required, otherwise API calls return 403
   - The status column should change to ✅ *Granted for [tenant]*

> ⚠️ **Application Permissions** mean the app can read **any mailbox in the tenant**. Use this only for tenants you administer; do not share the client secret.

### 2. Provide credentials

Two methods. The CLI tries env vars first, then falls back to macOS Keychain.

**Method A — Environment variables (any OS):**
```bash
export MS365_CLIENT_ID=...
export MS365_TENANT_ID=...
export MS365_CLIENT_SECRET=...
export MS365_USER_EMAIL=you@yourdomain.com
```

**Method B — macOS Keychain (recommended on Mac):**

ms365-cli reads Keychain directly via Apple's Security framework — no shell-out, no extra runtime dependency. You only need a one-time setup to populate the 4 entries (under `account=agent-secrets`).

Using built-in `security`:
```bash
for k in client-id tenant-id client-secret user-email; do
  read -rs -p "ms365-prod-$k: " v && echo \
  && security add-generic-password -s "ms365-prod-$k" -a agent-secrets -w "$v"
done
```

Or using the [`secret`](https://gist.github.com/zhaidewei/secret) wrapper (nicer prompt, same storage):
```bash
secret add ms365-prod-client-id
secret add ms365-prod-tenant-id
secret add ms365-prod-client-secret
secret add ms365-prod-user-email
```

### 3. Verify

```bash
ms365 search "test" -n 1
```
Should print one NDJSON line. If you get `403`, admin consent (step 6 above) was not granted.

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
