# molk

[English](README.md) · [**中文**](README.zh-CN.md)

为 LLM agent 优化的 Outlook 邮件 CLI —— 通过 Microsoft Graph API 读取 Microsoft 365 邮箱，避免在 HTML 噪声上烧 token。

写这个工具的原因：Outlook MCP server 把邮件 body 当作原始 HTML 透传（~85% 是 `MsoNormal` 这类 CSS 噪声），LLM agent（比如 Claude Code）每读一封邮件就要烧几千个 token。`molk` 把 HTML 转纯文本、裁剪字段，并消除 MCP server 的 schema 常驻成本。

> 名字由来：**`m`(ail) + `OLK`** —— `OLK` 是 Microsoft 用了二十多年的 Outlook 内部缩写（见 `.olk15` 配置文件、注册表 `HKCU\...\Office\...\Outlook\OLK`、Mac 端 `.olm` 归档格式都源于此）。

## 为什么不用 `pnp/cli-microsoft365`？

[`pnp/cli-microsoft365`](https://github.com/pnp/cli-microsoft365)（1.3k⭐）是 M365 管理 CLI 的事实标准，但它**不是** `molk` 的替代品 —— 两者坐在设计空间的两端：

| | `pnp/cli-microsoft365` | `molk` |
|---|---|---|
| Scope | 整个 M365（SharePoint / Teams / Outlook / OneDrive / Planner ...）—— 几百个命令 | Outlook 邮件，2 个命令 |
| 运行时 | Node.js 20+ | 单文件 3.7 MB 静态二进制 |
| 邮件 body | 原始 HTML（`{contentType: "html", content: "<html>…"}`），无 plain-text 选项 | html2text → 纯文本 |
| Auth | Delegated + Application | Application only |
| 目标用户 | 管理员、运维、CI 脚本 | LLM agent |
| Token 经济 | 无优化 | 一等公民 |

要管 SharePoint 站点、Teams 策略，用 `pnp/cli-microsoft365`。要让 agent 读邮箱不被 `MsoNormal` 烧 2k token，用 `molk`。

## 特性

- 🪶 **纯文本输出** —— html2text 干掉 `MsoNormal` 等 CSS；body 字节数 -85%
- 🔐 **凭据进 Keychain** —— 通过 Apple Security framework 直接读 macOS Keychain（不 shell-out、零 runtime 依赖），CI / Linux 走 env var fallback
- ⚡ **Token 缓存** —— access token 缓存到 `~/Library/Caches/molk/`，1 小时复用
- 📦 **3.7 MB 单二进制** —— 同等 Python MCP 要 50+ MB
- 🎯 **只有 2 个命令** —— `search` 和 `read`，拒绝功能蔓延

## 安装

```bash
git clone git@github.com:zhaidewei/molk.git
cd molk
cargo build --release
ln -sf "$PWD/target/release/molk" ~/.local/bin/molk  # 确保 ~/.local/bin 在 PATH
```

需要 Rust 1.75+。

## 配置

### 1. 注册 Entra (Azure AD) 应用

需要在你的 Microsoft 365 tenant 里注册一个 single-tenant 的 Entra 应用，并赋予 Microsoft Graph 的 Application Permissions。一次性配置，约 5 分钟。

1. 登录 [Azure Portal](https://portal.azure.com) → 进入 **Microsoft Entra ID**（旧名 Azure AD）
2. 左侧菜单 → **App registrations** → **New registration**
   - **Name**：`molk`（任意）
   - **Supported account types**：选 *Accounts in this organizational directory only (Single tenant)*
   - **Redirect URI**：留空（这是 server-to-server flow，不需要回调）
   - 点 **Register**
3. 创建后的 **Overview** 页面，复制：
   - **Application (client) ID** → 这就是 `MOLK_CLIENT_ID`
   - **Directory (tenant) ID** → 这就是 `MOLK_TENANT_ID`
4. 左侧 → **Certificates & secrets** → **Client secrets** → **+ New client secret**
   - **Description**：`molk`
   - **Expires**：推荐 24 个月（到期前要 rotate）
   - 点 **Add**
   - **立刻复制 *Value* 字段** —— 页面刷新后就再也看不到了 → 这就是 `MOLK_CLIENT_SECRET`
5. 左侧 → **API permissions** → **+ Add a permission**
   - 选 **Microsoft Graph** → **Application permissions**（注意不是 Delegated）
   - 搜索并勾选 `Mail.Read`
   - （可选）以后想发邮件就加 `Mail.Send`
   - 点 **Add permissions**
6. **Grant admin consent for [tenant]** ← 必须点这一步，否则 API 调用会返回 403
   - 状态列应该变成 ✅ *Granted for [tenant]*

> ⚠️ **Application Permissions** 意味着这个 app 可以读 tenant 里**任何邮箱**。强烈建议用 [Application Access Policy](https://learn.microsoft.com/en-us/graph/auth-limit-mailbox-access) 限定到自己的邮箱，并且 client secret 不要外传。

### 2. 配置凭据

两种方式，CLI 优先读 env var，失败回退到 Keychain。

**方式 A —— 环境变量（任何系统）：**
```bash
export MOLK_CLIENT_ID=...
export MOLK_TENANT_ID=...
export MOLK_CLIENT_SECRET=...
export MOLK_USER_EMAIL=you@yourdomain.com
```

**方式 B —— macOS Keychain（Mac 推荐）：**

`molk` 通过 Apple Security framework 直接读 Keychain，不 shell-out，runtime 没有额外依赖。你只需一次性把 4 个凭据写进 Keychain（`account=agent-secrets`）。

用 macOS 自带的 `security`：
```bash
for k in client-id tenant-id client-secret user-email; do
  read -rs -p "molk-prod-$k: " v && echo \
  && security add-generic-password -s "molk-prod-$k" -a agent-secrets -w "$v"
done
```

或者用 [`secret`](https://gist.github.com/zhaidewei/secret) 包装（输入交互更舒服，存储格式相同）：
```bash
secret add molk-prod-client-id
secret add molk-prod-tenant-id
secret add molk-prod-client-secret
secret add molk-prod-user-email
```

### 3. 验证

```bash
molk search "test" -n 1
```
应该输出一行 NDJSON。如果返回 `403`，是上面第 6 步的 admin consent 没点。

## 用法

```bash
# 按关键词搜索（subject + body）。NDJSON 一行一封到 stdout
molk search "invoice" -n 10

# 读一封邮件，纯文本 body
molk read AAMkAD...

# 读一封邮件，完整 JSON 输出（body 仍然 HTML-stripped）
molk read AAMkAD... --json
```

### Token 经济（数字）

典型「找邮件」工作流（1 次 search + 3 次 read）：

| 场景                          | 字节     |
|-------------------------------|----------|
| Outlook MCP（HTML body）      | ~20,800  |
| `molk`（纯文本）              | ~3,000   |
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
