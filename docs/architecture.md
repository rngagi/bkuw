# bkuw 架構與資料模型

## 系統形狀

React 負責 presentation、interaction、draft state 與 localization；Rust 負責 project lifecycle、filesystem、validation、SQLite、migrations、backup、locking 與 aggregate transactions。

```text
React UI
  ↓ typed adapter + Zod validation
Tauri commands
  ↓
Project/database module
  ↓
SQLite + project filesystem
```

Frontend 不可直接執行 SQL。所有 `invoke` 集中在 `src/lib/tauri.ts`，讓 command 名稱、DTO shape 與 error mapping 只有一個 seam。Project/database module 的外部 interface 提供 create/open/close project、settings、entry query/load/create/save/delete/restore 與 relation autocomplete；連線、SQL、normalization、backup 和 transaction 都留在 implementation 內。

## Project lifecycle

- 一次只允許一個 active project。
- 建立 project 時產生 `<name>.bkuw/project.sqlite` 與 `backups/`，遇到既有路徑不得覆寫。
- 開啟時 canonicalize 路徑、驗證目錄與 database identity/schema，再取得 exclusive project lock。
- migration 前以 SQLite-consistent 方法建立 timestamped backup；migration 失敗時保留原資料並回報 stable error code。
- 關閉 project 時先 flush pending save，再關閉 connection 與釋放 lock。
- Tauri main window 僅有 dialog、必要 core capability，以及 scope 嚴格限定為官方 ISO 639-3 registry URL 的 opener permission；不開放 shell、HTTP 或 broad filesystem plugin。Rust filesystem 操作限制在 active project canonical path。

## Command interface

主要 commands：

```text
create_project(request) -> ProjectSnapshot
open_project(path) -> ProjectSnapshot
close_project() -> void
update_project_settings(request) -> ProjectSnapshot
query_entry_summaries(query) -> EntrySummary[]
load_entry(id) -> LexicalEntry
create_entry() -> LexicalEntry
save_entry(aggregate, expectedRevision) -> LexicalEntry
delete_entry(id, expectedRevision) -> DeletedEntry
restore_entry(id) -> LexicalEntry
```

Errors 使用 `{ code, message, details? }`，其中 code 至少區分 invalid_project、project_locked、unsupported_schema、validation、not_found、revision_conflict、database、filesystem。UI 只顯示依 code 本地化的安全訊息；debug detail 保留給 log。

`save_entry` 接收 forms、senses、examples、example forms 與 relations 的完整 aggregate，在單一 transaction 內以 replace-diff strategy 寫入。`revision` 使用 optimistic concurrency 防止較舊 autosave 覆蓋新資料。

## SQLite schema

所有 IDs 使用 UUID，timestamps 使用 UTC RFC 3339。所有 connections 啟用 foreign keys、busy timeout，並使用適合單機桌面程式的 WAL mode。

核心 tables：

- `projects`：identity、name、ISO 639-3 language metadata、timestamps。
- `writing_systems`：project、name、type、script/language tags、display role、sort order、font。
- `metadata_options`：project-owned POS／semantic-domain reusable values 與 sort order。
- `lexical_entries`：project、notes、revision、timestamps、soft-delete timestamp。
- `entry_forms`：entry、writing system、NFC text、derived search key、metadata、sort order。
- `senses`：entry、gloss、definition、POS、semantic domain、sort order。
- `examples`：sense、translation、notes、sort order。
- `example_forms`：example、writing system、NFC text、sort order。
- `entry_relations`：source、optional target、relation type、fallback text、notes、sort order。
- `schema_migrations`：已套用的 migration versions。

Display-role constraints 保證 primary 恰有一個、secondary 最多一個且兩者不同。Initial project setup 可在同一 transaction 建立 writing systems 與 primary role，避免中間無效狀態。

Owned children 使用 `ON DELETE CASCADE`。Relation target 被永久移除時使用 `SET NULL` 並保留 fallback label。被 entry forms 或 example forms 引用的 writing system 使用 `RESTRICT`。Relation 不得 self-reference，並須有 target 或非空 fallback。

## Unicode 與搜尋

- 顯示文字在 Rust 寫入前正規化為 NFC。
- `entry_forms.search_key` 是可重建的衍生欄位：Unicode case fold、分解、移除 combining marks、再正規化。
- query 使用同一演算法，對 search key 做 substring matching；Chinese、Tibetan、Thai、IPA 等未折疊內容仍保留並可搜尋。
- 不假設 code point 等於 grapheme；character-level UI behavior 必須使用 grapheme-aware APIs。
- Example forms 在 Milestone 1 保存同樣的正規化文字，但不納入 entry-list search。

## Autosave 與刪除

- 有效 draft 變更經 debounce 後保存；同一 entry 的 saves 排序執行。
- DOM composition events 是 autosave boundary：`compositionstart` 清除 debounce timer，active composition 期間不得送出 save 或用 backend snapshot reset form，`compositionend` 後才重新排程。
- `Ctrl/Cmd+S`、entry/project 切換與 window close 會先 flush。
- Entry aggregate transaction 回傳成功時立即更新 inline live status，不等待非關鍵的 entry-list refresh；list refresh 在背景執行。Failure 保留 draft 與 dirty state，顯示 retryable localized error 與可展開 backend detail。
- Entry delete 先經 confirmation，之後設定 `deleted_at` 並從一般 query 排除；UI 提供 immediate Undo。完整 Trash manager 後續再做。

Frontend adapter 對 Rust unit response 接受 Tauri JSON `null`，再映射為 TypeScript `void`；這適用於 `close_project` 等 commands。Window close handler prevent default 後依序 flush、close session、destroy window，任一步失敗都保留視窗與 draft。

Entry forms 在 frontend 依 writing-system settings 自動補齊並固定排序；example 先建立 primary form，再允許加入尚未使用的 writing system。Phonemic／phonetic delimiter 是 presentation concern，不寫回 lexical text。Document-level input policy 透過既有及動態 controls 統一關閉 autocorrect、autocapitalize、autocomplete 與 spellcheck。

Migration 2 新增 `metadata_options`。舊 schema 開啟時仍遵守先建立一致性 SQLite backup、再於 transaction 套用 migration 的規則。

## Frontend structure

```text
src/
├── app/
├── components/ui/
├── features/projects/
├── features/settings/
├── features/entries/
├── i18n/
├── lib/
└── types/
```

React Hook Form 管理 entry aggregate draft，Zod 負責 frontend validation。React context 管理 active project/selection；Milestone 1 不引入 Zustand 或 TanStack Query。列表只 virtualize DOM，不在初版引入 server paging。

## Verification strategy

- Rust integration tests 透過 project/database module interface 使用 temporary project 與真實 SQLite。
- Vitest + React Testing Library 測互動、autosave、translations、validation 與 nested editors。
- WebdriverIO Tauri service 執行主要 desktop workflow smoke test。
- GitHub Actions 在 Windows x64、macOS arm64/x64 執行 checks、tests、build 與 unsigned bundle artifacts。
