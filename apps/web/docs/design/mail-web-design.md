# LifeTrace Mail Web 设计与实现方案

> 状态：Implementation Baseline（2026-09-23）  
> 目标仓库：LifeTraceManage/LifeTrace-web + LifeTraceManage/LifeTrace-cloud  
> 模块路由：`/mail`  
> 当前运行时：LifeTrace Cloud Mail（Rust/Axum + PostgreSQL + IMAP/SMTP/MIME）  
> 上游参考：Kurrier。当前 LifeTrace 仓库未 vendoring / copying Kurrier 源码；若未来实际引入或修改 Kurrier，再按其 AGPL-3.0-or-later 条款处理。

## 0. 2026-09-23 实现决策更新

代码核对发现 `LifeTrace-cloud` 已经存在完整的邮件领域基础设施：账号与凭据加密、IMAP/SMTP、MIME 解析、30 天初始回填、增量同步、IMAP IDLE、邮件缓存、附件读取和常驻 `mail_worker`。因此当前实现不再额外部署一套重复的 Kurrier Runtime，而是直接把这些能力收口成 LifeTrace Cloud Mail API。

当前实际链路：

```text
LifeTrace Web /mail
      ↓ 统一 LifeTrace Session + mail scopes
LifeTrace Cloud /api/v1/mail/*
      ├── Mail Account / Identity / Draft
      ├── Search / Star / Read / Move
      ├── MIME / Attachment
      └── SMTP Send
             ↑
      mail_worker
      ├── IMAP IDLE
      └── polling / backfill
             ↓
      External Mail Providers
```

Kurrier 继续作为邮件产品和协议实现的参考项目；只有在后续确认其具体模块能带来明确维护收益时才评估接入。这样避免同一台个人云服务器上同时维护两套 Mail Cache、两套账号模型和两套同步 Worker。

本轮已经落地：统一 Inbox/账号切换、Inbox/Sent/Drafts/Archive/Trash/Starred、正文搜索、HTML 安全渲染、账号绑定/测试/同步、Identity（Display Name / Reply-To / Signature / Default）、服务端 Draft、出站附件、Reply/Reply All/Forward、Read/Star/Move、Mail → Notes/Task/Calendar/Waiting Item 与 `entity.link` 来源追踪。

## 1. 背景

LifeTrace Web 采用“一个 Web 服务、一个登录、多个独立功能工作区”的整体架构。

```text
LifeTrace Web
├── /          Portal / 统一入口
├── /notes     Notes 工作区
├── /mail      Mail 工作区
├── /execute   Execute 工作区
├── /finance   Finance 工作区
└── /assets    Assets 工作区
```

Mail 不作为一个完全独立的网站，也不在 LifeTrace 主界面中长期挤占固定区域。

用户进入 `/mail` 后，应进入完整的 Mail Workspace。

所有 LifeTrace Web 模块共享：

- 同一个 LifeTrace 账号；
- 同一个 LifeTrace 登录态；
- 同一个顶层 Portal；
- 同一套基础设计语言；
- 同一个 LifeTrace Cloud 账号体系；
- 但各模块拥有自己的 Workspace、导航和业务模型。

---

## 2. 产品定位

LifeTrace Mail 定位为：

> **运行在 LifeTrace 内部的自托管多邮箱客户端与邮件聚合工作区。**

LifeTrace Mail **不是自建邮件服务器**。

第一阶段不负责：

- 提供 `@lifetrace.*` 邮箱地址；
- 自建完整 SMTP 收件服务器；
- 自建邮件投递信誉体系；
- 自建反垃圾邮件平台；
- 自建 DNS / SPF / DKIM / DMARC 托管系统。

LifeTrace Mail 主要负责连接现有邮箱体系：

```text
Gmail
Outlook
QQ Mail
163 Mail
Enterprise IMAP/SMTP
Other standard IMAP/SMTP providers
```

整体目标：

```text
External Mail Providers
        │
        ▼
   Kurrier-based
    Mail Runtime
        │
        ▼
   LifeTrace Mail
        │
        ├── Notes
        ├── Execute
        ├── Calendar
        ├── Waiting Item
        └── Agent
```

---

## 3. 开源方案选择

LifeTrace Mail 正式选择：

> **Kurrier 作为邮件功能的核心开源基座。**

主要原因：

- 支持标准 SMTP / IMAP；
- 已有 Mailbox discovery；
- 已有邮件同步 Worker；
- 已有账号、Identity、Mailbox 等领域模型；
- 已有 OIDC / SSO 能力；
- 已有 Management API；
- 已具备自托管邮件工作区所需的大量基础能力；
- 比从零实现 IMAP 同步、MIME 解析和邮件状态管理风险更低。

LifeTrace 不应重新实现 Kurrier 已经成熟处理的邮件基础设施。

---

## 4. Kurrier 的使用原则

Kurrier 不以 iframe 方式接入：

```text
LifeTrace
   ↓
iframe
   ↓
Kurrier
```

也不把 `/mail` 永久作为一个视觉上完全独立的第三方页面。

正确方向：

```text
Kurrier Source
      ↓
保留邮件核心能力
      ↓
改造认证 / Shell / UI
      ↓
增加 LifeTrace Integration
      ↓
LifeTrace Mail
```

目标是：

> **保留 Kurrier 的邮件基础设施，重新定义 LifeTrace Mail 的产品层与集成层。**

---

## 5. 统一认证

### 5.1 用户只登录一次 LifeTrace

Mail 不应要求第二套 LifeTrace 登录。

```text
/login
   ↓
LifeTrace Authentication
   ↓
/mail
   ↓
直接进入 Mail Workspace
```

不存在：

```text
LifeTrace Login
      ↓
Kurrier Login
      ↓
Mail
```

### 5.2 Kurrier 接入 LifeTrace Identity

优先使用 Kurrier 已有 OIDC / SSO 能力，将 LifeTrace 作为身份源。

目标关系：

```text
LifeTrace Identity
       │
       ▼
     OIDC
       │
       ▼
Kurrier User Mapping
       │
       ▼
   Mail Workspace
```

用户首次进入 Mail 时，如果 Kurrier 内部尚无对应用户，应通过自动 Provisioning 或 Management API 创建对应账户映射。

### 5.3 外部邮箱授权不是第二套平台登录

例如绑定 Gmail：

```text
LifeTrace Login
      ↓
Mail Settings
      ↓
Connect Gmail
      ↓
Google Authorization
      ↓
Mail account connected
```

这是“添加数据源”，不是重新登录 LifeTrace。

以后打开：

```text
/mail
```

无需再次登录 Gmail。

---

## 6. Mail Workspace

推荐桌面 Web 采用三栏结构：

```text
┌──────────────────────────────────────────────────────┐
│ ← LifeTrace         Mail         Search        User │
├────────────┬──────────────────┬──────────────────────┤
│ Compose    │                  │                      │
│            │                  │                      │
│ Inbox      │   Message List   │   Message Detail     │
│ Starred    │                  │                      │
│ Sent       │                  │                      │
│ Drafts     │                  │                      │
│ Archive    │                  │                      │
│ Trash      │                  │                      │
│            │                  │                      │
│ Accounts   │                  │                      │
└────────────┴──────────────────┴──────────────────────┘
```

设计目标：

- 桌面端高信息密度；
- 不照搬 Outlook 的复杂 Ribbon；
- 不使用过度卡片化 Dashboard；
- Mail 进入后就是完整邮件客户端；
- 只保留少量 LifeTrace 顶层导航元素。

移动端改为逐层导航：

```text
Mailbox
   ↓
Message List
   ↓
Message Detail
```

---

## 7. 第一阶段功能范围

第一阶段必须完成：

```text
Mail
├── Unified Inbox
├── Inbox
├── Starred
├── Sent
├── Drafts
├── Archive
├── Trash
├── Search
├── Compose
├── Reply
├── Reply All
├── Forward
├── Attachments
├── Accounts
├── Identities
└── Mailbox Sync
```

同时必须具备：

- IMAP 收件；
- SMTP 发件；
- Mailbox discovery；
- 初始历史邮件同步；
- 增量同步；
- 邮件已读状态同步；
- Flag / Star 同步；
- Folder / Mailbox 状态同步；
- 附件读取；
- HTML / Plain Text 邮件渲染；
- 基础 MIME 解析；
- 多邮箱账号支持。

---

## 8. 第一阶段暂不优先实现

为控制复杂度，第一阶段暂不优先：

- PGP；
- S/MIME；
- 高级 Sieve 管理；
- 复杂邮件规则编辑器；
- Shared Mailbox；
- 企业 Exchange 深度管理；
- 自建 SMTP server；
- 自建 Spam Engine；
- 营销邮件发送；
- Newsletter 平台；
- 邮件 CRM；
- 多人协同处理同一 Inbox。

这些能力可以在 Kurrier 原生能力和后续需求基础上逐步评估。

---

## 9. Account / Identity / Mailbox 三层模型

Mail 不应把邮箱账号、发件身份和文件夹揉成一个对象。

建议延续 Kurrier 的领域分层思想。

### 9.1 Account

表示一个真实的外部邮件连接。

例如：

```text
Gmail Account
Outlook Account
QQ Mail Account
Corporate IMAP Account
```

主要保存：

```text
id
user_id
provider
email
auth_type
encrypted_credentials
imap_host
imap_port
smtp_host
smtp_port
status
last_sync_at
created_at
updated_at
```

注意：

> 具体字段应尽量复用 Kurrier 现有数据模型，不要为了统一命名而无意义重写成熟结构。

### 9.2 Identity

Identity 表示发件身份。

一个 Account 未来可能对应多个发件 Identity。

例如：

```text
Account: user@example.com

Identities:
├── Xingxing Zhou <user@example.com>
└── Research <research@example.com>
```

Identity 主要管理：

- display name；
- from address；
- reply-to；
- signature；
- default identity。

### 9.3 Mailbox

表示邮箱中的逻辑文件夹：

```text
Inbox
Sent
Drafts
Archive
Trash
Spam
Custom Folder
```

Mailbox 应与外部 IMAP Folder 建立稳定映射。

---

## 10. 多邮箱账号

LifeTrace Mail 从数据模型上必须支持：

```text
LifeTrace User
│
├── Gmail
├── Outlook
├── QQ Mail
└── Corporate Mail
```

第一版 UI 即使先只验证少量 Provider，也不能把数据模型写死成：

```text
1 LifeTrace User = 1 Mail Account
```

必须支持：

```text
1 LifeTrace User = N Mail Accounts
```

---

## 11. Unified Inbox

LifeTrace Mail 应提供：

```text
All Inboxes
```

将多个账号 Inbox 聚合。

用户可以：

```text
All Inboxes
├── Gmail
├── Outlook
└── QQ
```

也可以切换某一个单独账户。

统一收件箱只是逻辑视图，不改变原邮箱服务器上的 Folder 结构。

---

## 12. 同步架构

邮件同步必须在服务端完成。

禁止：

```text
Browser
   ↓
直接 IMAP / SMTP
```

正确结构：

```text
Browser
   ↓
LifeTrace / Mail API
   ↓
Mail Runtime
   ↓
Kurrier Worker
   ↓
IMAP / SMTP Providers
```

原因：

- 浏览器不应持有 IMAP 密码；
- OAuth refresh token 不应暴露给前端；
- 邮件同步需要长期后台运行；
- IMAP IDLE 需要服务端常驻；
- 历史 backfill 需要后台任务；
- 多端访问需要统一同步状态。

---

## 13. 凭据安全

邮箱密码、应用专用密码、OAuth refresh token 等必须：

- 只在后端保存；
- 加密存储；
- 不通过常规 API 返回明文；
- 不写入前端 localStorage；
- 不进入前端日志；
- 不进入普通业务日志；
- 支持凭据更新和撤销。

浏览器最多知道：

```json
{
  "accountId": "xxx",
  "provider": "gmail",
  "email": "user@example.com",
  "status": "connected"
}
```

不应拿到原始 secret。

---

## 14. 邮件本地缓存

LifeTrace Mail 不应每次打开一封邮件都完全依赖实时 IMAP 查询。

推荐继续使用 Kurrier 已有的服务端缓存 / 同步模型：

```text
IMAP Provider
     ↓
Sync Worker
     ↓
Mail Cache
     ↓
Mail API
     ↓
LifeTrace Web
```

缓存主要负责：

- Message metadata；
- Thread 信息；
- Mailbox mapping；
- Flags；
- Search index；
- 同步游标；
- 必要的邮件正文缓存。

外部邮箱仍然是邮件事实来源。

---

## 15. 邮件详情

Message Detail 至少支持：

- From；
- To；
- Cc；
- Bcc（适用时）；
- Subject；
- Date；
- Plain text；
- HTML；
- Attachment；
- Reply；
- Reply All；
- Forward；
- Archive；
- Delete；
- Star；
- Mark read / unread。

HTML 邮件必须进行安全渲染处理。

至少应考虑：

- 禁止任意脚本执行；
- 防止恶意 HTML；
- 外部图片加载策略；
- 链接安全；
- iframe / embedded content 限制。

---

## 16. Compose

Compose 第一阶段支持：

```text
To
Cc
Bcc
Subject
Body
Attachment
Signature
Identity
```

编辑器支持 Rich Text，但底层应避免和整个 LifeTrace Notes Markdown 编辑器强行复用。

Mail Composer 与 Notes Editor 是两个不同业务场景。

---

## 17. Search

第一阶段支持：

- Subject；
- Sender；
- Recipient；
- Message body；
- Mailbox；
- Account；
- Date range。

后续可扩展：

```text
Keyword Search
       +
Semantic Search
       +
Agent Search
```

未来可以进入 LifeTrace Global Search：

```text
Global Search
├── Notes
├── Mail
├── Tasks
├── Assets
└── Files
```

---

## 18. Mail → Notes

这是 LifeTrace Mail 的核心增强能力之一。

邮件详情页增加：

```text
Save to Notes
```

生成：

```text
LifeTrace Note
├── title
├── selected / full mail content
├── sender
├── sent_at
└── source = mail://message/{messageId}
```

用户后续在 Notes 中可以通过 Source Link 回到原邮件。

---

## 19. Mail → Task

邮件详情页增加：

```text
Create Task
```

例如邮件：

> Please submit the report before Friday 18:00.

创建：

```text
Task
├── title = Submit report
├── deadline = Friday 18:00
└── source = mail://message/{messageId}
```

第一阶段可以手动确认字段。

后续 Agent 可以辅助解析：

- task title；
- due date；
- priority；
- project。

---

## 20. Mail → Calendar

对于包含会议、预约、时间地点信息的邮件：

```text
Create Calendar Event
```

生成：

```text
Calendar Event
├── title
├── start
├── end
├── location
└── source = mail://message/{messageId}
```

如果邮件中存在标准 ICS / calendar invitation，应优先解析标准结构，而不是只依赖 LLM。

---

## 21. Mail → Waiting Item

对于：

```text
“等对方回复”
“等待审批”
“等待 HR 反馈”
“等待订单处理”
```

提供：

```text
Add Waiting Item
```

生成：

```text
Waiting Item
├── subject
├── contact
├── created_at
├── expected_follow_up
└── source = mail://message/{messageId}
```

后续可以由 LifeTrace 自动提醒长时间未回复的事项。

---

## 22. Mail + Agent

邮件详情页保留：

```text
Ask Agent
```

第一阶段可支持：

- 总结邮件；
- 提取待办；
- 提取时间；
- 提取附件说明；
- 草拟回复；
- 查找相关 Notes；
- 查找相关历史邮件。

Agent 不直接替用户发送邮件。

发送动作仍需明确的用户操作。

后续如果设计自动化发送能力，应独立增加权限与确认机制。

---

## 23. Source Link

Mail 与 LifeTrace 其他模块之间统一采用稳定 Source Link。

建议：

```text
mail://message/{messageId}
mail://thread/{threadId}
mail://account/{accountId}
```

例如：

```text
Task
 └── source = mail://message/xxx
```

```text
Note
 └── source = mail://thread/yyy
```

这使不同模块之间可以建立可追踪关系，而不是复制一份邮件后失去来源。

---

## 24. LifeTrace Entity Link

如果现有 `entity.link` 能满足需求，优先使用：

```text
Mail Message
    ↓
entity.link
    ↓
Note / Task / Calendar / Waiting Item
```

如果当前 Entity Link 无法表达邮件领域关系，再新增 Mail Link abstraction。

避免每个模块独立发明一套关联系统。

---

## 25. 前端模块结构

建议：

```text
src/
├── app/
│   ├── auth/
│   ├── router/
│   └── layout/
│
├── portal/
│
├── modules/
│   ├── notes/
│   │
│   └── mail/
│       ├── pages/
│       │   ├── MailHome
│       │   ├── InboxPage
│       │   ├── MailboxPage
│       │   ├── SearchPage
│       │   └── AccountSettings
│       │
│       ├── components/
│       │   ├── MailSidebar
│       │   ├── MessageList
│       │   ├── MessageDetail
│       │   ├── Composer
│       │   ├── AccountSwitcher
│       │   └── MailActions
│       │
│       ├── api/
│       ├── stores/
│       ├── hooks/
│       └── routes/
│
└── shared/
```

具体迁移时应尽量复用 Kurrier 已有组件，不要为了满足目录命名而机械重写。

---

## 26. 路由

建议：

```text
/mail
/mail/inbox
/mail/starred
/mail/sent
/mail/drafts
/mail/archive
/mail/trash

/mail/account/:accountId
/mail/mailbox/:mailboxId

/mail/thread/:threadId
/mail/message/:messageId

/mail/search
/mail/settings/accounts
/mail/settings/identities
```

所有 `/mail/*`：

```text
requiresAuth = true
```

未登录：

```text
/mail
   ↓
/login?redirect=/mail
```

登录完成后返回 `/mail`。

---

## 27. Kurrier 与 LifeTrace Cloud 的边界

第一阶段不要为了“架构统一”而立即把 Kurrier 所有后端逻辑重写进 LifeTrace Cloud。

推荐分阶段融合。

### 初期

```text
LifeTrace Web
     │
     ├── LifeTrace APIs
     │
     └── Mail APIs
            ↓
        Kurrier Runtime
```

LifeTrace Auth 作为统一身份入口。

Kurrier 保留邮件核心：

- IMAP；
- SMTP；
- Sync Worker；
- Mail Cache；
- Search；
- Mailbox；
- Account；
- Identity。

### 中期

逐步统一：

- User mapping；
- File / Attachment；
- Entity Link；
- Global Search；
- Notifications；
- Observability；
- Deployment。

### 后期

只有当维护收益明确时，再评估：

> 是否把部分 Kurrier backend module 真正并入 LifeTrace Cloud。

不要一开始重写邮件协议核心。

---

## 28. 部署结构

第一阶段推荐仍然保持单机可部署。

例如：

```text
Docker Compose

├── lifetrace-web
├── lifetrace-cloud
├── kurrier / mail-runtime
├── mail-worker
└── required storage
```

在用户体验上仍然是一个 LifeTrace 服务：

```text
https://lifetrace.example.com/mail
```

内部有几个容器不影响“一个平台”的产品定义。

“一个服务”指统一入口、统一身份和统一产品体验，不要求所有进程必须强行塞进同一个容器。

---

## 29. 反向代理

推荐：

```text
Caddy / Nginx

/           → LifeTrace Web
/api/*      → LifeTrace Cloud
/mail-api/* → Mail Runtime
```

或者由 LifeTrace Cloud 提供统一 API Gateway。

最终 URL 对用户保持稳定。

---

## 30. 开源许可证处理

由于 Kurrier 使用 AGPL-3.0-or-later：

- 保留 Kurrier 原有 LICENSE；
- 保留必要版权声明；
- 不删除上游作者信息；
- 明确标记 LifeTrace 的修改；
- 对基于 Kurrier 修改并通过网络提供服务的对应代码，按照适用的 AGPL 条款提供源码；
- 第三方依赖许可证继续分别保留。

仓库中建议新增：

```text
THIRD_PARTY_NOTICES.md
```

记录：

```text
Kurrier
Project URL
Upstream commit / version
License
Modified areas
```

具体许可证合规如进入商业分发阶段，应再进行专门法律审查。

---

## 31. Kurrier 上游同步策略

不能把 Kurrier 代码复制进来以后完全失去上游关系。

建议保留：

```text
upstream = kurrier-org/kurrier
```

并记录：

- 初始 fork commit；
- LifeTrace 修改范围；
- 上游同步日期；
- 上游安全更新；
- IMAP / MIME / Provider compatibility 修复。

优先将 LifeTrace 修改集中在：

```text
Auth Adapter
LifeTrace Shell
LifeTrace Integration
Theme
Routes
```

尽量减少对邮件协议核心的侵入式修改。

这样未来更容易同步 Kurrier 的 bugfix。

---

## 32. LifeTrace 特有能力边界

LifeTrace 的创新部分主要放在：

```text
Mail
   ↓
LifeTrace Context
   ↓
Notes / Tasks / Calendar / Waiting / Agent
```

而不是放在重新实现：

```text
IMAP
SMTP
MIME
Mailbox sync
```

前者是 LifeTrace 的产品价值。

后者是成熟基础设施，应尽量复用 Kurrier。

---

## 33. 实现阶段

### Phase 1：Kurrier Baseline

目标：

> 先证明 Kurrier 可以稳定作为 LifeTrace 的 Mail Runtime。

完成：

- Kurrier 本地部署；
- IMAP 账号绑定；
- SMTP 发件；
- Inbox；
- Sent；
- Draft；
- Archive；
- Trash；
- Message Detail；
- Attachment；
- Search；
- Multi Account 基础验证。

验收：

```text
至少完成两个不同 Provider 的真实账号收发测试。
```

---

### Phase 2：LifeTrace Authentication

完成：

- LifeTrace Auth → Kurrier OIDC / SSO；
- 统一用户映射；
- 移除或隐藏独立 Kurrier 登录入口；
- `/mail` AuthGuard；
- 首次进入自动 Provisioning；
- Logout 行为统一。

验收：

> 用户登录一次 LifeTrace 后，可以直接进入 Mail。

---

### Phase 3：LifeTrace Mail Workspace

完成：

- LifeTrace Top Bar；
- 独立 Mail Layout；
- 三栏布局；
- Account Switcher；
- Unified Inbox；
- LifeTrace Theme；
- 路由统一到 `/mail/*`。

验收：

> 产品体验上不再像“跳转到了另一个系统”。

---

### Phase 4：LifeTrace Integration

完成：

- Mail → Notes；
- Mail → Task；
- Mail → Calendar；
- Mail → Waiting Item；
- Source Link；
- Entity Link；
- Ask Agent。

验收：

> 邮件可以成为 LifeTrace 中任务、知识和日程的数据入口。

---

### Phase 5：Platform Integration

完成：

- Global Search；
- Notification Center；
- Portal Recent；
- Mail unread count；
- Agent tools；
-统一 File Service；
-统一 Observability。

---

### Phase 6：Advanced Mail

按需求逐步评估：

- Rules；
- Sieve；
- PGP；
- Advanced Search；
- Semantic Search；
- Smart Categories；
- AI triage；
- Thread summary；
- Follow-up detection。

---

## 34. 第一阶段明确不做

为避免过度设计：

- 不重写 IMAP；
- 不重写 SMTP；
- 不重写 MIME parser；
- 不自己做邮件服务器；
- 不马上拆成完整微服务体系；
- 不把 Kurrier iframe 进来；
- 不给 Mail 再做一个 LifeTrace 账号；
- 不要求第一天就把 Kurrier 所有后端代码并入 LifeTrace Cloud；
- 不用 AI 替代标准邮件协议解析；
- 不默认让 Agent 自动发送邮件。

---

## 35. 与 Notes 的关系

已经确定的两个 LifeTrace Web 核心工作区：

```text
LifeTrace Web
│
├── Notes
│   ├── Memos 工程参考
│   ├── Obsidian UX
│   └── SilverBullet Knowledge Link
│
└── Mail
    └── Kurrier-based
```

二者建立双向关系：

```text
Mail
  └── Save to Notes
          ↓
        Notes

Notes
  └── Source Link
          ↓
         Mail
```

后续 Global Search 可以同时查询二者。

---

## 36. 总体架构

```text
                         LifeTrace Web
                              │
                         LifeTrace Auth
                              │
               ┌──────────────┼──────────────┐
               │                             │
            Portal                         /mail
                                             │
                                     Mail Workspace
                                             │
                 ┌───────────────────────────┼────────────────────────┐
                 │                           │                        │
              Inbox                       Compose                 Search
                 │                           │                        │
                 └──────────────┬────────────┴─────────────┬─────────┘
                                │                          │
                          Mail Runtime                 LifeTrace Links
                                │                          │
                            Kurrier                  ┌─────┼─────┐
                                │                    │     │     │
                     ┌──────────┼──────────┐       Notes Task Calendar
                     │          │          │
                   IMAP        SMTP     Sync Worker
                     │          │          │
                     └──────────┼──────────┘
                                │
                         External Mail
```

---

## 37. 最终产品定义

LifeTrace Mail 定义为：

> **一个运行在 LifeTrace Web 内部、基于 Kurrier 改造、支持多个外部邮箱账号统一管理，并与 Notes、Task、Calendar、Waiting Item 和 Agent 深度联动的自托管邮件工作区。**

技术上：

> **LifeTrace 统一登录 + Kurrier 邮件核心 + LifeTrace 集成层。**

产品上：

> **一个独立的 /mail Workspace。**

架构上：

> **复用成熟邮件基础设施，不重新发明 IMAP / SMTP；LifeTrace 的研发重点放在统一体验、统一身份以及跨模块信息流。**

最终目标不是重新实现 Gmail 或 Outlook，而是让 Mail 成为：

> **LifeTrace 中连接外部通信、知识、任务与执行系统的重要信息入口。**
