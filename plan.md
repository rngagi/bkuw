# bkuw 執行清單

規則：只有在該項完成條件成立且列出的驗證通過後才能勾選。若行為或架構改變，必須同步更新產品與架構文件。

本機驗證狀態（2026-08-12）：Milestone 1 功能、tests、desktop E2E、release build 與 unsigned macOS app／DMG bundle 已通過。GitHub Actions workflow 已建立；Windows x64 與 macOS Apple Silicon 的遠端執行結果須在 push 後確認，因此只保留該項未勾選。macOS Intel 不在支援與建置範圍內。

## 0. 文件轉換

- [x] 建立 `AGENTS.md`、`README.md`、`docs/product-spec.md`、`docs/architecture.md` 與本清單；逐節確認原規格均已保存後刪除舊計畫。
  - 完成條件：repo 不再包含 `bkuw_implementation_plan.md`，且新文件涵蓋產品原則、MVP、schema、UI、Unicode、backup、migration 與 roadmap。
  - 驗證：`rg --files | sort`、人工逐節核對。

## 1. Scaffold 與 app shell

- [x] 建立 Tauri 2 + React/Vite + TypeScript + pnpm 專案並鎖定工具版本。
  - 完成條件：dev/build scripts、Rust crate、Tauri config、Tailwind 與品質工具可運行。
  - 驗證：`pnpm check`、`pnpm tauri build --no-bundle`。
- [x] 建立 `en`／`zh-TW` localization 與可切換的 app shell。
  - 完成條件：OS locale detection、English fallback、persisted manual switch；無核心 hard-coded user strings。
  - 驗證：locale component tests 與兩語 smoke test。

## 2. Project 與 storage

- [x] 實作 create/open/close project、canonical-path validation 與 single-project lock。
  - 完成條件：不覆寫既有路徑；invalid/locked project 回傳 stable errors；close 釋放資源。
  - 驗證：Rust lifecycle integration tests。
- [x] 實作初始 SQLite migration、foreign-key constraints 與 version tracking。
  - 完成條件：schema 符合 architecture doc；transaction failure 不留下 partial data。
  - 驗證：migration、constraint、rollback tests。
- [x] 實作 migration 前 backup 與 reopen persistence。
  - 完成條件：備份為一致性 database；失敗不損壞原 project；重開後資料一致。
  - 驗證：backup/migration/reopen integration tests。

## 3. Project settings

- [x] 實作 project metadata 與動態 writing-system CRUD/reorder。
  - 完成條件：支援 name/type/script/language/font/order；已被使用的 writing system 不可刪除。
  - 驗證：Rust constraints tests 與 settings component tests。
- [x] 強制一個 primary、最多一個不同的 secondary writing system。
  - 完成條件：UI 與 database transaction 都拒絕無效組合。
  - 驗證：role validation tests。

## 4. Entry workspace

- [x] 建立可調寬度 two-pane workspace、搜尋列與 virtualized entry list。
  - 完成條件：左右獨立捲動；列表顯示 primary、secondary 與彙整 POS；keyboard selection 可用。
  - 驗證：workspace interaction tests。
- [x] 實作 entry create/load 與多 writing-system forms editor。
  - 完成條件：forms 依 writing-system order 顯示；文字以 NFC 保存；空 project/entry state 完整。
  - 驗證：aggregate load/save integration tests。
- [x] 實作 Unicode、case/diacritic-insensitive substring search。
  - 完成條件：`過`、`guò`、`guo`、IPA 均找到同一 entry且不改寫原文。
  - 驗證：Unicode search integration tests。

## 5. Senses 與 examples

- [x] 實作 ordered senses 與 sense-level POS。
  - 完成條件：可新增、編輯、刪除、排序 senses；entry 不保存第二份 POS。
  - 驗證：sense editor tests 與 persistence tests。
- [x] 實作 ordered examples、translation、notes 與多 writing-system example forms。
  - 完成條件：同一 sense 可有多 examples；每個 example 可保存原文、轉寫、IPA 等動態 forms。
  - 驗證：nested editor、reorder、rollback、reopen tests。

## 6. Relations 與可靠編輯

- [x] 實作 root/base free-text fallback、autocomplete、link 與 navigation。
  - 完成條件：target 或 fallback 至少一個；禁止 self-link；target 移除後 fallback 仍可顯示。
  - 驗證：relation constraints 與 navigation tests。
- [x] 實作 aggregate autosave、manual save、flush 與 revision conflict handling。
  - 完成條件：nested changes 原子保存；切換/關閉前 flush；失敗保留 dirty draft 並可 retry。
  - 驗證：autosave timing、rollback、stale revision tests。
- [x] 實作 soft delete 與 immediate Undo。
  - 完成條件：一般 query 排除 deleted entry；Undo 恢復資料與 relations。
  - 驗證：delete/restore integration 與 UI tests。
- [x] 實作 `Ctrl/Cmd+N`、`Ctrl/Cmd+F`、`Ctrl/Cmd+S`、`Ctrl/Cmd+Enter` 與可預測 tab order。
  - 完成條件：快捷鍵不攔截文字輸入中的無關行為，所有 controls 可用鍵盤操作。
  - 驗證：keyboard interaction tests。

## 7. Hardening 與 acceptance

- [x] 完成本地化、accessibility、loading/error/empty/locked states。
  - 完成條件：兩語無漏字串；長 English label 不破版；狀態不只靠顏色；controls 有 labels/focus。
  - 驗證：locale、accessibility component tests 與人工 keyboard walkthrough。
- [x] 建立 Chinese、Tibetan、Latin-script fixtures 與可選 demo project generator。
  - 完成條件：一般新 project 保持空白；fixtures 覆蓋 dynamic writing systems 與 Unicode examples。
  - 驗證：fixture-driven integration tests。
- [x] 建立 Windows x64／macOS Apple Silicon CI 與未簽名 bundle artifacts。
  - 完成條件：Windows x64、macOS Apple Silicon 執行 checks/tests/build；產出 installer/bundles，不發布 release、不建置 macOS Intel。
  - 驗證：GitHub Actions workflow 成功。
- [x] 完成 Milestone 1 本機最終驗收。
  - 完成條件：依 `docs/product-spec.md` 完整走過 create→configure→edit forms/senses/examples/relations→search→autosave→reopen→locale switch。
  - 驗證：`pnpm check && pnpm test && pnpm test:rust && pnpm tauri build --no-bundle`，加上 desktop smoke E2E。

## 8. 使用者回饋 hardening

- [x] 修正 project／window close 的 Rust unit response adapter 與 pending-save flush。
- [x] 加入 ISO 639-3 說明、官方 registry 系統瀏覽器連結、同名 project modal 與新 project writing-system onboarding。
- [x] 以 migration 2 加入 reusable POS／semantic-domain metadata，sense editor 改用下拉選單。
- [x] Entry forms 依 project writing systems 自動補齊；example 預設 primary form 並禁止重複 writing system。
- [x] Phonemic／phonetic presentation 使用 `/…/`／`[…]`，且不改寫儲存文字。
- [x] 所有 input／textarea 關閉 autocorrect、autocapitalize、autocomplete 與 spellcheck。
- [x] Autosave 顯示 inline success status；failure 顯示具體原因與 backend detail。
- [x] IME 組字期間暫停 autosave 與表單 reset；`compositionend` 後才保存，並保持輸入焦點與候選字流程。
- [x] Entry transaction 成功後立即顯示已儲存，entry-list 背景 refresh 不阻塞狀態更新。
- [x] 說明 root/base 未連結詞形用途，entry delete 加入二次確認並保留 Undo。
  - 驗證：component/adapter/input-policy tests、migration/backup/reopen tests、雙語 1280×720 browser walkthrough、desktop E2E 與 release build。

## 9. v0.2 export milestone

- [x] 以 migration 3 加入 nullable analysis language 與 versioned project export profile。
  - 完成條件：舊 project 保持 null；profile 可保存、關閉、重開；migration 前仍建立一致性 backup。
  - 驗證：migration/profile persistence 與完整 Rust suite。
- [x] 實作 rngagi-corpus v0.3 九欄 CSV preview 與 export。
  - 完成條件：一 sense 一列、UTF-8 無 BOM、RFC 4180/CRLF、ICU4X deterministic sort、blocking/warning diagnostics、stale token、temporary sibling atomic replacement。
  - 驗證：exact golden CSV、Unicode quoting/newline/comma、loss diagnostics、soft-delete/stale/overwrite Rust tests。
- [x] 實作可攜 XeLaTeX source project 與 Overleaf ZIP。
  - 完成條件：五個 source files、完整 TeX escaping、grapheme sections、portable fonts、Rust reverse index；ZIP 不含 PDF/log/aux。
  - 驗證：`exports_portable_xelatex_project_and_overleaf_zip` 與 ZIP member assertion。
- [x] 實作 optional local PDF runner。
  - 完成條件：偵測 PATH/macOS/Windows paths、隔離 build、兩次 `-no-shell-escape` XeLaTeX、120 秒 timeout、缺少引擎仍成功、失敗保留 diagnostics。
  - 驗證：fake engine success/missing/failure/timeout test，以及 `cargo test ... portable_xelatex_template_compiles_with_the_real_engine -- --ignored`。
- [x] 實作英文／台灣繁中 Export wizard 與 ISO 15924 help。
  - 完成條件：format→profile→preview→destination→result；preview 前 flush；POS mapping、issue navigation、missing-XeLaTeX Overleaf flow、二次覆寫確認、精確 opener capabilities。
  - 驗證：ExportDialog/settings tests 與 TypeScript check。
- [x] 完成 v0.2 文件與相容性限制。
  - 完成條件：更新 README/product/architecture/roadmap；新增 CSV contract 與 bilingual export guide；明載沒有跨 repository automated contract test。
  - 驗證：人工文件 review 與 contract wording search。
- [x] 擴充 desktop E2E 與 portable-template CI。
  - 完成條件：desktop fixture 匯出 exact CSV、LaTeX/ZIP；有 XeLaTeX 的 CI job 編譯真實 template；人工 Overleaf upload walkthrough 記錄成功。
  - 驗證：`pnpm test:e2e:build && pnpm test:e2e`、本機 real-XeLaTeX ignored test；portable-template GitHub Actions job 已建立，遠端結果待 push 後確認。Overleaf upload 屬人工驗收，依雙語指南執行且不得自動上傳資料。
- [x] 完成 v0.2 最終驗收並統一升版。
  - 完成條件：package、Cargo、Tauri 都是 `0.2.0`；Windows x64、macOS Apple Silicon checks/build/bundle 成功。
  - 驗證：`pnpm check && pnpm test && pnpm test:rust && pnpm test:e2e:build && pnpm test:e2e && pnpm tauri build --no-bundle`，加上 GitHub Actions。

## 10. v0.2.1 autosave focus hotfix

- [x] Autosave 完成後維持目前巢狀輸入欄位、focus、selection 與游標位置。
  - 完成條件：autosave success 不 reset entry aggregate；只同步 revision/timestamp，背景 entry-list refresh 不把 saved entry prop 灌回 editor。
  - 驗證：`EntryEditor` autosave focus regression test、App workflow tests、`pnpm check && pnpm test && pnpm test:rust`。
- [x] package、Cargo、Cargo lock 與 Tauri app version 一致升為 `0.2.1`。
  - 驗證：版本搜尋與 Tauri build。

## 11. v0.2.2 XeLaTeX diagnostic path hotfix

- [x] XeLaTeX compile failure／timeout 後，在雙語錯誤區顯示保留的 `diagnostic.log` 完整路徑。
  - 完成條件：僅 `latex_compile`／`latex_timeout` 顯示 backend detail；Windows 路徑可完整選取、複製，其他錯誤細節不外洩。
  - 驗證：`ExportDialog` Windows diagnostic-path regression test、`pnpm check && pnpm test && pnpm test:rust`。
- [x] 修正 Windows 關閉主視窗時被 Tauri capability 拒絕的問題。
  - 完成條件：main window close request 仍先 flush autosave、close project，再以最小 `core:window:allow-destroy` 權限完成關窗；不授權其他視窗。
  - 驗證：main-window capability regression test、Tauri capability schema/build validation。
- [x] package、Cargo、Cargo lock 與 Tauri app version 一致升為 `0.2.2`。
  - 驗證：版本搜尋與 Tauri build。
- [x] 以 version tag 自動建立 Windows x64／macOS Apple Silicon Draft GitHub Release。
  - 完成條件：全部 CI jobs 成功後才建立；四處 version 必須符合 tag；只上傳一個 NSIS、一個 DMG 與 `SHA256SUMS.txt`；release notes 含雙語 unsigned 安裝提醒並接續 GitHub generated notes。
  - 驗證：workflow／release-note YAML parse、local version/installers collection shell checks，以及 `v0.2.2` tag 的 GitHub Actions／Draft Release assets。

## 12. v0.2.3 managed portable fonts

- [x] 建立 Rust font-manager deep module 與固定 catalog。
  - 完成條件：官方固定 commit/release、archive/member SHA-256、license、staging/manifest/cache verification；frontend 不直接執行 HTTP 或 filesystem。
  - 驗證：missing、checksum mismatch、cache corruption、namespaced export/license Rust tests，`pnpm check`。
- [x] LaTeX/PDF preview 與 export 改用 bkuw-managed fonts。
  - 完成條件：TeX Gyre Termes 缺少或 invalid 時 fatal；輸出使用 project-relative paths；folder/ZIP/temporary build 都含 required fonts/licenses，不查詢 OS fonts。
  - 驗證：fatal preview、portable folder/ZIP member 與 real-XeLaTeX tests。
- [x] Phonemic／phonetic writing systems 固定使用 Charis SIL。
  - 完成條件：Rust 忽略 IPA 類型的 preset override；Export wizard 顯示固定設定；preview 將 Charis SIL 列入 required packs。
  - 驗證：Rust font selection 與 ExportDialog component tests。
- [x] 完成雙語 font-pack status、下載與 retry 流程。
  - 完成條件：missing/installed/invalid/mandatory 狀態完整本地化；下載錯誤可重試；成功後自動重新 preview。
  - 驗證：English／zh-TW translations、ExportDialog download/retry test。
- [x] package、Cargo、Cargo lock 與 Tauri app version 一致升為 `0.2.3`，完成全部本機與遠端驗收。
  - 驗證：`pnpm check && pnpm test && pnpm test:rust && pnpm test:e2e:build && pnpm test:e2e && pnpm tauri build --no-bundle`；GitHub Actions run `31614974863` 的 Portable XeLaTeX、Windows x64 與 macOS Apple Silicon 全部成功；公開 `v0.2.3` Release 含 NSIS、DMG 與 `SHA256SUMS.txt`。

## 13. 移除未使用的 script-specific font packs

- [x] 完整移除 Noto Serif Thai／Tibetan managed packs 與 presets。
  - 完成條件：catalog、下載來源、hash、Rust／TypeScript enum、Export UI、雙語翻譯、E2E 與文件均不再宣稱或使用兩個專用 packs；一般 Thai／Tibetan lexical data 與 Unicode 搜尋能力不受影響。
  - 驗證：catalog regression test、repository search、`pnpm check && pnpm test && pnpm test:rust && pnpm test:e2e:build && pnpm test:e2e && pnpm tauri build --no-bundle`，以及 real-XeLaTeX portable-template test。

## 14. v0.3 dictionary ordering and LaTeX refresh

- [x] 建立 project alphabet 與共用自動排序。
  - 完成條件：可選 writing system；一行一 element；longest-match 支援 `ng`／`ch`；空 alphabet 使用 ICU4X；entry list 顯示 headings。
  - 驗證：`custom_alphabet_sorts_multigraphs_and_supplies_section_labels`、ordering unit test、`pnpm check && pnpm test && pnpm test:rust`。
- [x] 建立 entry section override 與二次確認。
  - 完成條件：只改小標，完整表記仍在 section 內自然排序；不改拼寫／搜尋；manual mode 時停用。
  - 驗證：Rust regroup test 與 EntryEditor confirmation test。
- [x] 建立 opt-in 完全自訂拖拉排序。
  - 完成條件：headings／entries 可拖拉及鍵盤上移下移；可匯入 headings 或不建立 headings；新 entries 標示 pending；切回 auto 保留 layout。
  - 驗證：manual layout Rust test、camelCase Tauri contract test、SortOrderDialog／missing-layout recovery component tests、`test.bkuw` 副本的移動→儲存→重開 desktop walkthrough，以及 migration/backup tests。
- [x] 重整 XeLaTeX template 並加入 optional direct root/base related entries。
  - 完成條件：與 workspace 共用排序／小標；相關詞只顯示一層 incoming live relations；可選 none/root/base/both；改善行距、例句標記、section 與 related block。
  - 驗證：renderer／舊 profile 相容性 tests、real XeLaTeX compile、兩頁 PDF visual QA，以及 `pnpm check && pnpm test && pnpm test:rust && pnpm tauri build --no-bundle`。
- [x] 改善 export snapshot 與長時間匯出的回應性。
  - 完成條件：aggregate 使用固定組數 bulk queries；preview/export/font/XeLaTeX blocking 工作在背景執行；取得一致 snapshot 後釋放 project lock；CSV 不預先掃描 fonts/XeLaTeX；等待期間顯示雙語階段進度。
  - 驗證：bulk nested aggregate regression、ExportDialog lazy-check/progress tests、`pnpm check && pnpm test && pnpm test:rust && pnpm tauri build --no-bundle`。
- [x] 讓 LaTeX pronunciation 只顯示於主要詞頭右側。
  - 完成條件：pronunciation form 不再重複出現在其他表記 metadata；headword／pronunciation 不可選用相同 writing system；舊重複 profile 可安全 normalize。
  - 驗證：Rust renderer／settings contract tests、ExportDialog 重複選項測試與真實 XeLaTeX compile。
- [x] 人工檢查排序與 LaTeX 結果後，將四處版本一致升為 `0.3.0`。
  - 完成條件：使用者確認後 bump、commit、push 並以 `v0.3.0` tag 建立 Windows x64／macOS Apple Silicon release。
  - 驗證：版本一致性檢查、`pnpm check && pnpm test && pnpm test:rust && pnpm tauri build --no-bundle`、真實 XeLaTeX compile；遠端 installer 與 release 由 tag workflow 驗收。
