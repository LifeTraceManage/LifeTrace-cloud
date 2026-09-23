# LifeTrace Notes Web 设计与实现方案

> 状态：Implementation Baseline（2026-09-24）  
> 目标仓库：LifeTraceManage/LifeTrace-web + LifeTraceManage/LifeTrace-cloud  
> 模块路由：`/notes`

## 0. 2026-09-24 实现状态

当前 Notes 已不再只是设计稿。已落地的核心能力包括：

- 独立 `/notes` Workspace，并与 Mail 共用 LifeTrace 基础主题、统一 Session 和 Workspace Shell；
- Cloud-backed Note / Folder / Tag / Tag Relation / Wiki Relation / Revision；
- Markdown 编辑、Vditor 本地草稿恢复与 Cloud 自动保存；
- Inbox / All / Recent / Favorites / Trash；
- 嵌套 Folder（`parentFolderId`）与文件夹重命名、移动、删除迁移；
- Tags、全文搜索与 Global Search 精确打开指定 Note；
- `[[Wiki Link]]` / Alias 自动补全、Links / Backlinks；
- Properties：status / source / aliases，并支持 YAML frontmatter 导入；
- 多标签页、Command Palette、附件上传/下载/删除；
- Notes Graph；
- Daily Note；
- 版本历史：保存前快照、最多保留 50 个版本、可恢复；
- Notes → Task，使用 `entity.link` 保留 `notes://note/{id}` 来源；
- Calendar → Notes；
- Mail → Notes 已在 Mail Workspace 中实现。

当前仍属于后续增强而非本轮阻塞项的能力：Semantic Search、AI Assistant、复杂模板系统、多人实时协作与 Canvas。

实现继续坚持一个原则：Notes 数据走现有 LifeTrace Sync / File / Entity Link，不再额外拆一个 Notes Backend。

## 1. 背景

LifeTrace Web 不再设计为把所有功能长期堆叠在同一个工作台中的“大一统页面”，而是作为统一 Web 平台，通过不同路由进入相对独立的功能工作区。

```text
LifeTrace Web
├── /          Portal / 统一入口
├── /notes     Notes 工作区
├── /mail      Mail 工作区
├── /execute   Execute 工作区
├── /finance   Finance 工作区
└── /assets    Assets 工作区
```

所有工作区共享：

- 同一个 LifeTrace 账号；
- 同一个登录状态；
- 同一个 LifeTrace Cloud；
- 同一套基础设计语言；
- 但拥有各自独立的页面布局、交互模型与功能边界。

LifeTrace Notes 是第一批重点拆出的 Web 工作区之一。

---

## 2. 产品定位

LifeTrace Notes 不定位为简单的“云端便签”，而定位为：

> **LifeTrace 内部的个人知识库与信息组织中心。**

设计来源：

```text
Memos
  ↓
工程实现、Web CRUD、基础笔记能力

Obsidian
  ↓
文件树、标签页、双向链接、
Backlinks、Properties、Graph、
Command Palette 等交互理念

SilverBullet
  ↓
Markdown / Wiki Link /
知识链接能力参考
```

最终不是直接复制任何一个项目，而是：

> **以 Memos 作为工程参考，以 Obsidian 作为主要交互参考，以 SilverBullet 作为知识链接能力参考，构建属于 LifeTrace 的 Notes。**

---

## 3. 核心设计原则

### 3.1 一个 LifeTrace 登录

Notes 不维护自己的账号系统。

```text
/login
   ↓
LifeTrace Authentication
   ↓
/notes
```

只要 LifeTrace Session 有效，就可以直接进入 Notes。

不存在二次登录：

```text
LifeTrace Login
      ↓
Notes Login
```

所有 `/notes/*` 路由复用统一 AuthGuard。

### 3.2 一个 Web 服务，独立工作区

Notes 与其他功能仍属于同一个 LifeTrace Web 服务，但 `/notes` 拥有自己的完整 Workspace。

不采用全站固定超级侧边栏承载所有业务。

推荐布局：

```text
┌──────────────────────────────────────────────────┐
│ ← LifeTrace     Notes          Search      User │
├────────────┬─────────────────────────────────────┤
│ Explorer   │ Tab A │ Tab B │ Tab C              │
│            ├─────────────────────────────────────┤
│ Inbox      │                                     │
│ Notes      │                                     │
│ Projects   │              Editor                 │
│ Research   │                                     │
│ Daily      │                                     │
│            │                                     │
├────────────┤                                     │
│ Tags       │                                     │
│ Backlinks  │                                     │
└────────────┴─────────────────────────────────────┘
```

### 3.3 Markdown 为核心数据格式

笔记正文统一以 Markdown 为核心存储格式，而不是把富文本 DOM 或 HTML 作为唯一数据源。

优势：

- 可移植；
- 易导出；
- 易版本管理；
- 易全文搜索；
- 易供 Agent 阅读；
- 未来可以兼容本地 Markdown 文件。

---

## 4. 第一阶段功能范围

第一版实现：

```text
Notes
├── All Notes
├── Recent
├── Favorites
├── Inbox
├── Folder
├── Tags
├── Markdown Editor
├── Search
├── Attachments
├── Wiki Link
├── Backlinks
└── Trash
```

第一阶段不优先实现：

- 复杂插件系统；
- Canvas；
- 高级 Graph 分析；
- 多人实时协作；
- 数据库视图；
- 复杂模板系统；
- AI 自动写作。

---

## 5. 页面信息架构

默认路由：

```text
/notes
```

默认进入最近使用或 Inbox。

整体布局分为：

```text
Top Bar
   +
Navigation Sidebar
   +
Workspace
```

左侧导航：

```text
Notes
├── Inbox
├── All Notes
├── Recent
├── Favorites
│
├── Folders
│   ├── Work
│   ├── Research
│   ├── Personal
│   └── ...
│
├── Tags
└── Trash
```

Folder 支持嵌套：

```text
Research
├── Quadrotor
│   ├── SMF
│   ├── MPC
│   └── Papers
└── AI
```

---

## 6. 编辑器设计

### 6.1 Markdown

示例：

```markdown
# Tube MPC

当前需要解决：

- 固定 Tube 过保守
- 扰动集合在线更新
- 递归可行性

关联：

[[Set Membership Filter]]
[[Constrained Zonotope]]
```

### 6.2 编辑模式

第一阶段提供：

- Editing；
- Preview。

后续实现类似 Obsidian 的 Live Preview，在保持 Markdown 源数据的前提下减少语法符号干扰。

### 6.3 多标签页

支持同时打开多个 Note：

```text
┌──────────────┬──────────────┬───────────────┐
│ Tube MPC     │ SMF Theory   │ Paper Notes   │
└──────────────┴──────────────┴───────────────┘
```

以下 UI 状态第一阶段可以保存在浏览器本地：

- `opened_tabs`
- `active_tab`
- `tab_order`

不要求第一阶段跨端同步。

---

## 7. Wiki Link 与双向链接

### 7.1 Wiki Link

支持：

```text
[[Note Name]]
```

例如：

```markdown
SMF 可以结合 [[Tube MPC]] 降低控制保守性。
```

输入 `[[` 时提供 Note 自动补全。

### 7.2 Backlinks

假设 Note A 包含：

```text
[[B]]
```

则 Note B 自动显示：

```text
Backlinks

Referenced by:
- A
```

Backlink 不单独人工维护，而由 `note.link` 关系反向查询得到。

---

## 8. Properties

参考 Obsidian Properties。

基础属性：

- title
- created_at
- updated_at
- tags
- favorite
- status
- source
- aliases

应兼容类似 YAML Frontmatter 的语义：

```yaml
---
status: research
tags:
  - MPC
  - SMF
source: paper
---
```

第一阶段不要求直接向用户暴露 YAML 编辑，但底层模型应可映射这些结构化属性。

---

## 9. Folder 与 Tag

Folder 用于层级组织，Tag 用于跨目录分类，两者不能混为一套模型。

示例标签：

```text
#MPC
#SMF
#research
```

标签路由：

```text
/notes/tag/:tag
```

---

## 10. 搜索

第一阶段支持：

- 标题搜索；
- 正文全文搜索；
- Tag 搜索；
- Folder 搜索。

后续升级：

```text
Keyword Search
      +
Semantic Search
```

Notes Search 后续接入 LifeTrace Global Search。

---

## 11. Command Palette

保留统一命令面板架构。

建议快捷键：

```text
Ctrl / Cmd + P
```

第一阶段命令：

```text
> New Note
> Search Notes
> Open Note
> Move Note
> Delete Note
> Add Tag
```

未来扩展：

```text
> Ask AI
> Create Task
> Link Asset
> Create Calendar Event
```

Command Palette 将成为 Notes 与 LifeTrace 其他模块联动的重要入口。

---

## 12. 附件

支持：

- Image
- PDF
- Document
- Archive
- Other files

Markdown 内使用逻辑引用，例如：

```markdown
![architecture](attachment://xxx)
```

实际文件交给 LifeTrace File Service 管理。

```text
Note
 ├── Markdown
 └── Attachments
       ↓
 LifeTrace File API
```

附件文件本体不直接存入 Notes 业务表。

---

## 13. 数据模型

LifeTrace Cloud 建议新增独立 Notes Domain。

### 13.1 note.note

建议字段：

```text
id
user_id
title
content
folder_id
favorite
created_at
updated_at
deleted_at
```

为未来 Local-first / Sync 预留：

```text
version
revision
sync_status
```

具体字段可根据现有 Cloud Sync 协议统一设计。

### 13.2 note.folder

```text
id
user_id
name
parent_id
sort_order
created_at
updated_at
```

通过 `parent_id` 支持 Folder 嵌套。

### 13.3 note.tag

```text
id
user_id
name
```

关联表：

```text
note.note_tag
```

### 13.4 note.link

```text
id
source_note_id
target_note_id
link_text
created_at
```

主要支撑：

- Wiki Link；
- Backlinks；
- Graph。

### 13.5 Attachment

优先复用现有：

```text
file.metadata
entity.link
```

如现有 Entity Link 无法表达 Notes 所需关系，再增加 `note.attachment`。

---

## 14. API 设计

统一使用：

```text
/api/notes/*
```

### Note

```http
GET    /api/notes
POST   /api/notes

GET    /api/notes/:id
PATCH  /api/notes/:id
DELETE /api/notes/:id
```

### Folder

```http
GET    /api/notes/folders
POST   /api/notes/folders

PATCH  /api/notes/folders/:id
DELETE /api/notes/folders/:id
```

### Tag

```http
GET    /api/notes/tags
POST   /api/notes/:id/tags
DELETE /api/notes/:id/tags/:tag
```

### Link

```http
GET /api/notes/:id/links
GET /api/notes/:id/backlinks
```

### Search

```http
GET /api/notes/search?q=
```

---

## 15. 前端模块结构

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
│   └── notes/
│       ├── pages/
│       │   ├── NotesHome
│       │   ├── NoteEditor
│       │   ├── TagPage
│       │   ├── SearchPage
│       │   └── TrashPage
│       │
│       ├── components/
│       │   ├── NoteExplorer
│       │   ├── NoteTabs
│       │   ├── MarkdownEditor
│       │   ├── BacklinkPanel
│       │   ├── PropertyPanel
│       │   └── CommandPalette
│       │
│       ├── api/
│       ├── stores/
│       ├── hooks/
│       ├── parser/
│       └── routes/
│
└── shared/
```

Notes 应保持模块内部独立，避免与其他工作区形成强耦合。

---

## 16. 路由

建议：

```text
/notes
/notes/inbox
/notes/all
/notes/recent
/notes/favorites
/notes/n/:noteId
/notes/folder/:folderId
/notes/tag/:tag
/notes/search
/notes/trash
```

所有 `/notes/*`：

```text
requiresAuth = true
```

未登录时统一跳转 LifeTrace `/login`，登录成功后返回原目标路由。

---

## 17. 开源方案的使用方式

### 17.1 Memos

Memos 不作为独立服务永久嵌入 LifeTrace。

不采用：

```text
LifeTrace
  ↓
iframe
  ↓
Memos
```

也不把 `/notes` 长期反向代理到一个独立 Memos 应用。

正确方式：

```text
Memos Source
      ↓
分析
      ↓
提取可复用设计与代码
      ↓
LifeTrace Notes
```

重点参考：

- Markdown Editor；
- Note CRUD；
- Tag；
- Search；
- Attachment；
- Frontend architecture；
- API interaction。

### 17.2 Obsidian

Obsidian 主要作为 UX 参考，不直接依赖其应用本体。

重点参考：

- Explorer；
- Tabs；
- Wiki Link；
- Backlinks；
- Properties；
- Graph；
- Command Palette。

### 17.3 SilverBullet

主要参考：

- Wiki Link parsing；
- Markdown linking；
- Knowledge graph；
- Link resolution。

重点研究 `[[Page Name]]` 如何解析为稳定的 Note 关系。

---

## 18. 与 LifeTrace 其他模块联动

最终目标：

```text
                Notes
                  │
       ┌──────────┼──────────┐
       │          │          │
    Execute     Mail       Assets
       │          │          │
    Finance    Agent      Calendar
```

### Mail → Notes

邮件支持：

```text
Save to Notes
```

生成 Note 并保存来源关联。

### Notes → Execute

选中文字后支持：

```text
Create Task
```

自动创建 Task，并记录 Note Source。

### Assets → Notes

资产可以关联：

- 使用记录；
- 维修记录；
- 配置说明；
- 购买与保修资料。

---

## 19. Agent 集成

未来 Agent 应暴露或调用类似能力：

```text
notes.search
notes.get
notes.create
notes.update
notes.link
```

例如用户询问：

> 找一下之前 Tube MPC 递归可行性的研究结论。

Agent 可以：

```text
Search Notes
     ↓
Retrieve related notes
     ↓
Answer
```

因此 Markdown、Link、Properties 和 Search 是 Notes 的核心基础设施，而不是单纯 UI 功能。

---

## 20. Graph View

Graph UI 不作为第一阶段阻塞项。

只要底层已有：

```text
note.note
+
note.link
```

即可在后续实现 Graph View。

第一阶段优先保证 Link 数据正确，第二阶段再做可视化。

---

## 21. Local-first 预留

虽然第一版主要是 Web，但数据模型必须考虑未来：

```text
Web
Desktop
Android
```

最终可发展为：

```text
Local Notes DB
      ↓
Outbox
      ↓
LifeTrace Cloud
```

Note ID 应采用稳定 UUID，并兼容 LifeTrace 现有同步协议、版本字段和冲突处理机制。

---

## 22. 实现阶段

### Phase 1：Notes Core

完成：

- LifeTrace 统一登录；
- Notes Workspace；
- Folder；
- Note CRUD；
- Markdown Editor；
- Search；
- Tags；
- Favorites；
- Trash。

目标：

> 完成一个可日常使用的 Web Notes Core。

### Phase 2：Knowledge System

完成：

- `[[Wiki Link]]`；
- Backlinks；
- Properties；
- Tabs；
- Command Palette；
- Attachments。

目标：

> 形成类似 Obsidian 的基础个人知识管理能力。

### Phase 3：LifeTrace Integration

完成：

- Mail → Note；
- Note → Task；
- Asset → Note；
- Calendar → Note；
- Global Search。

目标：

> 让 Notes 成为 LifeTrace 的知识中心。

### Phase 4：Advanced Knowledge

完成：

- Graph View；
- Semantic Search；
- AI Assistant；
- Templates；
- Daily Notes；
- Version History。

---

## 23. 第一阶段明确不做

为避免把项目重新做重：

- 暂不做插件市场；
- 暂不做复杂实时协作；
- 暂不做完整 Obsidian Clone；
- 暂不做复杂 Canvas；
- 暂不做独立 Notes 账号；
- 暂不拆 Notes 微服务；
- 暂不单独部署 Notes Backend。

---

## 24. 总体架构

```text
                         LifeTrace Web
                              │
                         LifeTrace Auth
                              │
              ┌───────────────┼───────────────┐
              │                               │
           Portal                           Notes
                                              │
                                     /notes Workspace
                                              │
                      ┌───────────────────────┼─────────────┐
                      │                       │             │
                   Editor                   Search       Knowledge
                      │                       │             │
                 Markdown                 Full Text      Links
                      │                                     │
                      ├───────────────┐                    │
                      │               │                    │
                  Attachment        Tags             Backlinks
                      │
                      ▼
                LifeTrace Cloud
                      │
            ┌─────────┼───────────┐
            │         │           │
          Notes     Files       Entity Link
```

---

## 25. 最终产品定义

LifeTrace Notes 定义为：

> **一个运行于 LifeTrace Web 内部、以 Markdown 为核心、支持文件夹、标签、Wiki Link、双向链接与知识关联的个人知识管理工作区。**

技术上：

> **一个服务、一个登录、一个 LifeTrace Cloud。**

产品上：

> **一个独立 Notes Workspace。**

实现上：

> **借鉴 Memos 工程能力、Obsidian 交互设计与 SilverBullet 知识链接思想，但不直接绑定任何一个完整产品。**

最终目标不是做另一个 Memos，也不是复制 Obsidian，而是让 Notes 成为：

> **LifeTrace 所有信息、任务、邮件、资产与 Agent 能力之间的知识中枢。**
