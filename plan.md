# Milestone 1 執行清單

規則：只有在該項完成條件成立且列出的驗證通過後才能勾選。若行為或架構改變，必須同步更新產品與架構文件。

本機驗證狀態（2026-08-12）：Milestone 1 功能、tests、desktop E2E、release build 與 unsigned macOS app／DMG bundle 已通過。GitHub Actions workflow 已建立；Windows x64 與 macOS arm64/x64 的遠端執行結果須在 push 後確認，因此只保留該項未勾選。

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
- [ ] 建立 Windows/macOS CI 與未簽名 bundle artifacts。
  - 完成條件：Windows x64、macOS arm64/x64 執行 checks/tests/build；產出 installer/bundles，不發布 release。
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
