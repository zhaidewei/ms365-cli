# ms365-cli

[English](README.md) · [**中文**](README.zh-CN.md)

为 LLM agent 优化的 Outlook CLI —— 通过 Microsoft Graph API 读取 Microsoft 365 邮箱，避免在 HTML 噪声上烧 token。

写这个工具的原因：Outlook MCP server 把邮件 body 当作原始 HTML 透传（~85% 是 `MsoNormal` 这类 CSS 噪声），LLM agent（比如 Claude Code）每读一封邮件就要烧几千个 token。`ms365-cli` 把 HTML 转纯文本、裁剪字段，并消除 MCP server 的 schema 常驻成本。

## 特性

- 🪶 **纯文本输出** —— html2text 干掉 `MsoNormal` 等 CSS；body 字节数 -85%
- 🔐 **凭据进 Keychain** —— macOS Keychain 一等公民（通过 [`secret`](https://gist.github.com/zhaidewei/secret) 包装），CI / Linux 走 env var
- ⚡ **Token 缓存** —— access token 缓存到 `~/Library/Caches/ms365-cli/`，1 小时复用
- 📦 **3.7 MB 单二进制** —— 同等 Python MCP 要 50+ MB
- 🎯 **只有 2 个命令** —— `search` 和 `read`，拒绝功能蔓延

## 安装

```bash
git clone git@github.com:zhaidewei/ms365-cli.git
cd ms365-cli
cargo build --release
ln -sf "$PWD/target/release/ms365" ~/.local/bin/ms365  # 确保 ~/.local/bin 在 PATH
```

需要 Rust 1.75+。

## 配置

### 1. 注册 Entra (Azure AD) 应用

需要在你的 Microsoft 365 tenant 里注册一个 single-tenant 的 Entra 应用，并赋予 Microsoft Graph 的 Application Permissions。一次性配置，约 5 分钟。

1. 登录 [Azure Portal](https://portal.azure.com) → 进入 **Microsoft Entra ID**（旧名 Azure AD）
2. 左侧菜单 → **App registrations** → **New registration**
   - **Name**：`ms365-cli`（任意）
   - **Supported account types**：选 *Accounts in this organizational directory only (Single tenant)*
   - **Redirect URI**：留空（这是 server-to-server flow，不需要回调）
   - 点 **Register**
3. 创建后的 **Overview** 页面，复制：
   - **Application (client) ID** → 这就是 `MS365_CLIENT_ID`
   - **Directory (tenant) ID** → 这就是 `MS365_TENANT_ID`
4. 左侧 → **Certificates & secrets** → **Client secrets** → **+ New client secret**
   - **Description**：`ms365-cli`
   - **Expires**：推荐 24 个月（到期前要 rotate）
   - 点 **Add**
   - **立刻复制 *Value* 字段** —— 页面刷新后就再也看不到了 → 这就是 `MS365_CLIENT_SECRET`
5. 左侧 → **API permissions** → **+ Add a permission**
   - 选 **Microsoft Graph** → **Application permissions**（注意不是 Delegated）
   - 搜索并勾选 `Mail.Read`
   - （可选）以后想发邮件就加 `Mail.Send`
   - 点 **Add permissions**
6. **Grant admin consent for [tenant]** ← 必须点这一步，否则 API 调用会返回 403
   - 状态列应该变成 ✅ *Granted for [tenant]*

> ⚠️ **Application Permissions** 意味着这个 app 可以读 tenant 里**任何邮箱**。所以只在你自己管理的 tenant 用，client secret 不要外传。

### 2. 配置凭据

两种方式，CLI 优先读 env var，失败回退到 Keychain。

**方式 A —— 环境变量（任何系统）：**
```bash
export MS365_CLIENT_ID=...
export MS365_TENANT_ID=...
export MS365_CLIENT_SECRET=...
export MS365_USER_EMAIL=you@yourdomain.com
```

**方式 B —— macOS Keychain（Mac 推荐）：**

需要 [`secret`](https://gist.github.com/zhaidewei/secret) 包装脚本（在 macOS `security` 命令上薄封一层，把所有条目放到 `account=agent-secrets` 的命名空间下）：

```bash
secret add ms365-prod-client-id     "Entra app client_id"
secret add ms365-prod-tenant-id     "M365 tenant id"
secret add ms365-prod-client-secret "Entra app client secret"
secret add ms365-prod-user-email    "Target mailbox UPN"
```

### 3. 验证

```bash
ms365 search "test" -n 1
```
应该输出一行 NDJSON。如果返回 `403`，是上面第 6 步的 admin consent 没点。

## 用法

```bash
# 按关键词搜索（subject + body）。NDJSON 一行一封到 stdout
ms365 search "invoice" -n 10

# 读一封邮件，纯文本 body
ms365 read AAMkAD...

# 读一封邮件，完整 JSON 输出（body 仍然 HTML-stripped）
ms365 read AAMkAD... --json
```

### Token 经济（数字）

典型「找邮件」工作流（1 次 search + 3 次 read）：

| 场景                          | 字节     |
|-------------------------------|----------|
| Outlook MCP（HTML body）      | ~20,800  |
| `ms365` CLI（纯文本）         | ~3,000   |
| **降幅**                      | **−86%** |

加上你还消除了 ~700 token 的 MCP tool schema —— 那玩意是常驻 input context 的，每次对话都吃。

## Auth 模型

用 Microsoft Graph **Application Permissions**（client credentials flow）。这个 CLI 是设计给*你自己的*邮箱用的，配你自己拥有的 single-tenant Entra app。**不要**对你不管理的邮箱用。

## 限制

- 只读（暂不支持发 / 草稿 / 回复 —— 故意控制范围，scope creep 是大敌）
- 只支持 Application Permissions（不支持 Delegated User flow）
- 只在 Microsoft 365 Business / Exchange Online 测过（个人 `@outlook.com` 账号没测过）

## License

MIT —— 见 [LICENSE](LICENSE)。
