# bkuw 執行清單

本文件集中維護最近工作、尚待完成的驗收與候選 backlog。程式變更歷史見 [CHANGELOG.md](CHANGELOG.md)，發布狀態以 [GitHub Releases](https://github.com/rngagi/bkuw/releases) 為準。

規則：只有完成條件成立，且列出的驗證通過後才能勾選。行為或架構改變時，同步更新 `docs/product-spec.md` 與 `docs/architecture.md`。

## 目前基線與最近工作

現行功能以 [產品規格](docs/product-spec.md) 與 [架構](docs/architecture.md) 為準；歷史版本摘要已整併至 [變更紀錄](CHANGELOG.md)，不再維護重複的功能清單。

- `c452786`：義項／例句改用 WebM／Opus，加入程式內錄音；驗證紀錄與剩餘人工驗收見下節。
- `4df892b`：更新啟動畫面，字型背景檢查與語言選單旁的管理入口；已隨提交更新產品與架構文件。
- `2d9335f`、`14ad8c7`、`0d82852`：移除音訊建置 Python 依賴，修正 Windows MSYS2 工具鏈與 Actions runtime 路徑。

以上記錄程式已提交的內容，不代表外部驗收或正式發布完成。下列已勾選項目與測試數字沿用既有驗證紀錄，本次文件整理不新增驗收勾選。

啟動畫面字樣動畫已經使用者確認為 UI 規範例外，並同步記錄於 `AGENTS.md` 與產品規格；必須保留 reduced-motion 支援，其餘介面仍遵守原有動畫限制。

## 逐步匯出與 LaTeX 安裝引導

- [x] 五步驟匯出、本機 PDF 優先、必要套件／字型檢查、官方安裝器交接與雙語安裝／Overleaf 教學。驗證：`pnpm check`、`pnpm test`、`pnpm test:rust`。
- [x] 本機驗證：desktop E2E、含繁中／IPA／圖片的真實 XeLaTeX 編譯及 `pnpm tauri build --no-bundle`。
- [ ] 外部人工驗收：Windows x64／macOS Apple Silicon 官方安裝精靈與安裝後離線編譯、Overleaf 匯入含繁中／IPA／圖片的 ZIP。

## 義項／例句音檔與程式內錄音

- [x] 義項／例句匯入與錄音統一儲存 WebM／Opus，移除 MP3 相容層；上傳提示只顯示接受格式。驗證：`pnpm check`、`pnpm test`（95 passed）、`pnpm test:rust`（72 passed，1 個既有 XeLaTeX 測試 ignored）。
- [x] 本機 WebView 真實 MediaRecorder、WebM 播放／拖曳 E2E（完整 desktop E2E 4 passed）與 `pnpm tauri build --no-bundle`。
- [ ] Windows x64 CI／實機驗收：隨附工具建置、所有格式匯入、播放／拖曳及離線安裝包。
- [ ] Windows x64 實機麥克風權限、錄製、試聽與儲存，以及 macOS 真實麥克風人工試聽；使用真實詞彙與例句確認音質。

## Cloudflare 公開辭典網站

- [x] 發佈精靈、Token credential、Migration 8、publication snapshot、Workers Static Assets、R2 media Worker、正式網站 template 與雙語 UI。驗證：`pnpm check`、`pnpm test`（101 passed）、`pnpm test:rust`（81 passed，1 個既有 XeLaTeX 測試 ignored）。
- [x] 本機驗證：`pnpm test:e2e:build`、`pnpm test:e2e`（4 passed）、`pnpm tauri build --no-bundle`，並開啟 build 供人工測試。
- [ ] 使用自己的 Cloudflare 帳號完成首次發佈與一次差異更新；驗證 workers.dev、搜尋、deep-link、亮暗、info、writing systems、notes、relations、圖片、兩種 audio 與舊媒體清理。

## 候選 backlog

以下尚未承諾版本或優先順序；開始實作前須先確認 scope 與 acceptance criteria。

- [ ] 將 CSV 匯入既有專案，以及 versioned bkuw → rngagi-corpus upload workflow。
- [ ] bkuw／rngagi-corpus cross-repository contract fixture 與 CI。
- [ ] Regex search。
- [ ] IPA helper、tags、filters 與 duplicate detection。
- [ ] Trash／backup manager。
- [ ] Production signing、Apple notarization 與 auto-update。

Cloud sync、collaboration、mobile、ASR、ELAN-style timeline、waveform segmentation、Git syncing，以及未經使用者主動觸發的自動上傳 lexical data 仍屬明確排除範圍。Publish corpus website 所需的 Cloudflare authorization、部署與 media upload 是已列入 backlog 的唯一雲端例外。

## 驗證命令

一般行為變更：

```bash
pnpm check
pnpm test
pnpm test:rust
```

Milestone／release 前另執行：

```bash
pnpm test:e2e:build
pnpm test:e2e
pnpm tauri build --no-bundle
```

Release 前使用 `pnpm release:prepare -- <version>` 同步 package、Cargo、Cargo lock 與 Tauri app version；遠端僅驗收 Windows x64 與 macOS Apple Silicon，公開版本以 GitHub Releases 為準。
