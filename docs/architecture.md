# bkuw 架構與資料模型

## 系統形狀

React 負責 presentation、interaction、draft state 與 localization；Rust 負責 project lifecycle、filesystem、validation、SQLite、migrations、backup、locking、font-pack supply chain 與 aggregate transactions。

```text
React UI
  ↓ typed adapter + Zod validation
Tauri commands
  ↓
Project/database module
  ↓
SQLite + project filesystem
```

Frontend 不可直接執行 SQL。所有 `invoke` 集中在 `src/lib/tauri.ts`，讓 command 名稱、DTO shape 與 error mapping 只有一個 seam。Project/database module 的外部 interface 提供 create/open/close project、settings、entry query/load/create/save/delete/restore 與 export snapshot；連線、SQL、normalization、backup 和 transaction 都留在 implementation 內。Export module 是 deep module：公開 preview/run/detect 行為，內部封裝 corpus flattening、ICU4X sorting、TeX rendering/escaping、ZIP、atomic write 與 XeLaTeX process。

## Project lifecycle

- 一次只允許一個 active project。
- 建立 project 時產生 `<name>.bkuw/project.sqlite` 與 `backups/`，遇到既有路徑不得覆寫。
- 開啟時 canonicalize 路徑、驗證目錄與 database identity/schema，再取得 exclusive project lock。
- migration 前以 SQLite-consistent 方法建立 timestamped backup；migration 失敗時保留原資料並回報 stable error code。
- 關閉 project 時先 flush pending save，再關閉 connection 與釋放 lock。
- Tauri main window 僅有 open/save dialog、必要 core capability，以及 scope 嚴格限定為官方 ISO 639-3、Unicode ISO 15924、Overleaf project／compiler help URL 的 opener permission；不開放 shell、HTTP 或 broad filesystem plugin。Project database 操作限制在 active canonical project；export 只操作使用者經 dialog 選定的目的地。

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
save_export_settings(settings) -> ExportSettingsV1
preview_export(kind) -> ExportPreview
export_project(request) -> ExportResult
detect_xelatex() -> TexEngineStatus
list_font_packs() -> FontPackStatus[]
install_font_pack(packId) -> FontPackStatus
```

Errors 使用 `{ code, message, details? }`，其中 export 另穩定區分 `export_validation`、`export_stale`、`export_filesystem`、`latex_compile`、`latex_timeout`，字型管理另使用 `font_download`、`font_integrity`、`font_filesystem`、`font_unknown`。UI 顯示依 code 本地化的安全訊息；compile failure/timeout 的 detail 指向保留的 diagnostic log，frontend 只針對這兩個 code 將完整路徑顯示為可選取文字，不把其他內部 error details 外洩。

Main window 的 close request 由 React 攔截，先 flush entry autosave、關閉 active project session，再呼叫 Tauri `destroy()` 完成真正關窗。Capability 僅對 `main` window 額外授權 `core:window:allow-destroy`；這是 `core:default` 未包含、Windows 會強制檢查的必要權限。

`save_entry` 接收 forms、senses、examples、example forms 與 relations 的完整 aggregate，在單一 transaction 內以 replace-diff strategy 寫入。`revision` 使用 optimistic concurrency 防止較舊 autosave 覆蓋新資料。

## SQLite schema

所有 IDs 使用 UUID，timestamps 使用 UTC RFC 3339。所有 connections 啟用 foreign keys、busy timeout，並使用適合單機桌面程式的 WAL mode。

核心 tables：

- `projects`：identity、name、ISO 639-3 language metadata、timestamps。
- `projects.analysis_language`：nullable `zh-TW`／`en`；舊專案 migration 後仍為 null。
- `export_settings`：project-owned versioned JSON profile；v0.2 固定 version 1。
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
- Autosave success 不以 `reset(savedAggregate)` reconcile。Editor 只原地同步 Rust 管理的 `revision`／`updatedAt` 並更新 committed snapshot，避免 `useFieldArray` 重新產生 keys、重建巢狀 controls 與移走 focus／selection；真正切換或重新載入 entry 時才 reset aggregate。
- `Ctrl/Cmd+S`、entry/project 切換與 window close 會先 flush。
- Entry aggregate transaction 回傳成功時立即更新 inline live status，不等待非關鍵的 entry-list refresh；list refresh 在背景執行。Failure 保留 draft 與 dirty state，顯示 retryable localized error 與可展開 backend detail。
- Entry delete 先經 confirmation，之後設定 `deleted_at` 並從一般 query 排除；UI 提供 immediate Undo。完整 Trash manager 後續再做。

Frontend adapter 對 Rust unit response 接受 Tauri JSON `null`，再映射為 TypeScript `void`；這適用於 `close_project` 等 commands。Window close handler prevent default 後依序 flush、close session、destroy window，任一步失敗都保留視窗與 draft。

Entry forms 在 frontend 依 writing-system settings 自動補齊並固定排序；example 先建立 primary form，再允許加入尚未使用的 writing system。Phonemic／phonetic delimiter 是 presentation concern，不寫回 lexical text。Document-level input policy 透過既有及動態 controls 統一關閉 autocorrect、autocapitalize、autocomplete 與 spellcheck。

Migration 2 新增 `metadata_options`。Migration 3 新增 `projects.analysis_language` 與 `export_settings`。舊 schema 開啟時仍遵守先建立一致性 SQLite backup、再於 transaction 套用 migration 的規則。

## Export architecture

`ProjectSession` 建立只含 live entries 的完整 `ExportSnapshot`。Preview 以 snapshot + format 的 SHA-256 token 綁定資料；真正匯出前重新建立 snapshot，token 不同即回傳 `export_stale`。React 不讀 SQL 或 filesystem，所有 DTO 由 `src/types/domain.ts` 的 Zod schema 驗證。

CSV renderer 固定 rngagi-corpus v0.3 九欄。ICU4X 依 profile language tag 排 primary form，entry UUID 與 sense order 是 deterministic tie-breakers。Writer 使用 UTF-8、無 BOM、CRLF 及 RFC 4180 quoting。輸出先寫同層 temporary sibling；Unix 使用 replace rename，Windows 使用 `MoveFileExW` 的 replace/write-through flags，避免留下半成品。

LaTeX renderer 從零建立通用 XeLaTeX source，不複製 `docs/main.tex` 的授權巨集。所有 user text 經集中 escaping；writing-system font macros 使用純字母 control sequence與 project-relative font paths。Reverse index 由 Rust 排序並直接產生 `hyperlink`／`pageref`，不使用 makeindex。

Font manager 是另一個 deep module。固定 catalog 只包含 TeX Gyre Termes、Charis SIL、Noto Serif 與 Noto Serif CJK TC，並記錄 pack ID、上游固定 commit/release、HTTPS URL、archive members、逐檔與 archive SHA-256、版本、LaTeX faces 與授權檔。下載先進 app-local staging directory；只有 archive 與每個 extracted/downloaded file 全部通過雜湊驗證，才以 manifest 啟用 cache。cache 每次使用前依 manifest 重驗，損毀 pack 視為 invalid。React 不接觸網路或 filesystem，只能列出狀態與請求安裝；Rust HTTP client 只能使用 catalog 內建 URL。

TeX Gyre Termes 是所有 LaTeX/PDF export 的 mandatory base pack，缺少或 invalid 時 preview 產生 fatal blocking issue。分析語言與每個 writing system 依 profile/script 決定其他必要 packs；phonemic／phonetic 類型不接受 preset override，固定解析為 Charis SIL。需要的字型檔與相應 license 都放進 `fonts/<pack-id>/`，LaTeX folder 與 Overleaf ZIP 因此不依賴 OS font registry。

ZIP 打包 `main.tex`、`entries.tex`、`reverse-index.tex`、`.latexmkrc`、bilingual `README.md` 以及 `fonts/` 下的必要 font/license files，不含 PDF/log/aux。PDF runner 從 PATH、macOS TeX path 與 Windows 常見路徑找 XeLaTeX，把完整 sources tree 複製到 temporary build directory，兩次執行 `-no-shell-escape -interaction=nonstopmode -halt-on-error -file-line-error`，每次最多 120 秒。成功只複製 PDF；失敗/timeout 保留 source project 與 `diagnostic.log`。

CSV 的外部相容契約見 `docs/corpus-csv-contract.md`。目前沒有跨 repository 自動 contract test；`rngagi-corpus` 版本變更必須人工重驗與更新 golden fixture。

## Frontend structure

```text
src/
├── app/
├── components/ui/
├── features/projects/
├── features/settings/
├── features/entries/
├── features/export/
├── i18n/
├── lib/
└── types/
```

React Hook Form 管理 entry aggregate draft，Zod 負責 frontend validation。React context 管理 active project/selection；Milestone 1 不引入 Zustand 或 TanStack Query。列表只 virtualize DOM，不在初版引入 server paging。

## Verification strategy

- Rust integration tests 透過 project/database module interface 使用 temporary project 與真實 SQLite。
- Vitest + React Testing Library 測互動、autosave、translations、validation 與 nested editors。
- WebdriverIO Tauri service 執行主要 desktop workflow smoke test。
- GitHub Actions 在 Windows x64 與 macOS Apple Silicon 執行 checks、tests、build，並上傳 unsigned installer/bundle artifacts；不建立 macOS Intel 產物。`v*` tag 通過全部 jobs 後，受限 `contents: write` 的 final job 驗證四處 version、產生 SHA-256 checksums，並建立含 NSIS／DMG 與自動 changelog 的 Draft Release；發布前保留人工確認閘門。
