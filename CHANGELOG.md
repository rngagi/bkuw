# bkuw 變更紀錄

本文件記錄程式變更，不宣告版本已發布；公開版本、發布日期與安裝包以 [GitHub Releases](https://github.com/rngagi/bkuw/releases) 為準。現行行為見 [產品規格](docs/product-spec.md)，驗收狀態見 [執行清單](plan.md)。歷史條目中的 MP3 儲存與舊發布流程只描述當時實作。

## 0.6.2 版本準備後的變更

### 中文

- 義項與例句加入程式內錄音，停止後可試聽、重錄、取消或儲存；匯入與錄音統一轉為單聲道 WebM／Opus 64 kbps VBR、48 kHz，新增 WebM 來源支援。移除舊 MP3 儲存相容層，不建立 WAV 播放副本。音檔仍不包含在 CSV／LaTeX／PDF 匯出中。
- 更新啟動畫面與隨附 Pacifico 字型的字樣呈現；支援減少動態效果偏好。字型檢查改為背景執行，語言選單旁提供狀態與管理入口，不再阻擋建立或開啟專案。
- 音訊工具建置移除 Python 依賴；Windows 使用指定的 MSYS2 MINGW64，CI／release 取得 setup 回傳的實際安裝位置。

對應提交：`2d9335f`、`14ad8c7`、`0d82852`、`c452786`、`4df892b`。啟動畫面字樣動畫已經使用者確認為 UI 規範例外，並同步更新 `AGENTS.md` 與產品規格。

### English

- Added in-app recording for senses and examples, with preview, re-record, cancel and save. Imports and recordings now use mono WebM/Opus at 64 kbps VBR / 48 kHz, including WebM source support. Removed legacy MP3 storage compatibility; no WAV playback copies are created. Audio remains excluded from CSV, LaTeX and PDF exports.
- Refreshed the startup screen with a bundled Pacifico wordmark and reduced-motion support. Font checks now run in the background, with status and management beside the language selector, without blocking project creation or opening.
- Removed Python from audio builds and resolved the configured MSYS2 MINGW64 location on Windows, including the setup action's actual runtime path in CI and release builds.

## 0.6.2

### 中文

- 義項與例句可加入多個 WAV、MP3、M4A/AAC、FLAC、OGG/Opus 或 AIFF 音檔；bkuw 會在本機離線轉成單聲道 64 kbps MP3。
- 新增批次匯入、逐檔進度與錯誤、播放／暫停、拖曳進度及刪除操作，並完整支援英文與台灣繁中介面。
- 音檔保存在 project-local `media/audio`，讀取時驗證路徑、檔案大小與 SHA-256；刪除義項或例句時同步清理附件，詞條 Undo 仍會保留音檔。
- Windows x64 與 macOS Apple Silicon 安裝包隨附固定版本、完整性驗證且可離線使用的 FFmpeg／LAME 工具、授權及對應原始碼。
- 減少 GitHub Actions 重複檢查，取消同一 pull request 的過期 CI，並只讓 exact version commit 觸發 Draft Release 建置。
- 程式內錄音、音檔匯出，以及在 corpus CSV、LaTeX 或 PDF 中包含音檔尚未提供。

### English

- Senses and examples can now hold multiple WAV, MP3, M4A/AAC, FLAC, OGG/Opus, or AIFF files; bkuw converts them locally and offline to mono 64 kbps MP3.
- Added batch import with per-file progress and errors, playback and pause, seeking, deletion, and complete English and Taiwan Traditional Chinese UI coverage.
- Audio is stored under the project-local `media/audio` directory and verified by path, size, and SHA-256 when read; deleting a sense or example cleans up its attachments while entry Undo preserves them.
- Windows x64 and macOS Apple Silicon installers bundle fixed, integrity-checked FFmpeg/LAME tools, licenses, and corresponding source archives for offline use.
- Reduced duplicate GitHub Actions work, cancel outdated CI runs for the same pull request, and allow only the exact version commit to trigger Draft Release builds.
- In-app recording, audio export, and audio in corpus CSV, LaTeX, or PDF output are not included yet.

## 0.6.1

### 中文

- 修正桌面應用程式與 Windows/macOS 安裝程式仍使用舊 icon 的問題。
- 重新產生 Tauri 的 PNG、ICO、ICNS、Windows、Android 與 iOS icon 資產。

### English

- Fixed desktop app and Windows/macOS installer bundles still using the old icon.
- Regenerated the Tauri PNG, ICO, ICNS, Windows, Android, and iOS icon assets.

## 0.6.0

### 新功能

- 匯出流程改為五個步驟：選擇輸出、設定內容、檢查準備狀態、確認並匯出、查看結果。
- 本機 PDF 匯出會檢查 XeLaTeX、必要 TeX 套件、bkuw 管理的字型與最小編譯結果。
- 缺少 LaTeX 編譯器時，可從 App 下載並驗證 Windows x64 TeX Live 或 macOS Apple Silicon MacTeX，再開啟官方安裝精靈。
- 提供 Windows PowerShell、macOS shell 安裝腳本與雙語安裝說明。
- Overleaf 流程加入 ZIP 匯入、XeLaTeX 編譯器、`main.tex`、重新編譯與 PDF 下載的官方教學連結。
- 安裝下載支援進度、取消、重試、完整性驗證與既有 TeX 環境修復指引。

### 修正與維護

- LaTeX 環境檢查會區分缺少編譯器、缺少套件、編譯失敗與逾時，並保留診斷紀錄。
- 匯出精靈在返回設定後保留輸入內容，但會要求重新檢查，避免使用過期的 preview。
- PDF 未成功產生時，不會顯示為 PDF 匯出完成，並保留 LaTeX 原始碼與 Overleaf ZIP。
- 更新應用程式 icon。此項變更來自 commit `4b38efb`。
- 版本與 Windows x64、macOS Apple Silicon 發布流程更新為 `0.6.0`。

## 早期里程碑摘要

以下由原 roadmap 整併，保留當時功能與流程的演進；目前發布流程以 [發布文件](docs/distribution.md) 為準。

### v0.1 / Core editor

Local-first project lifecycle、dynamic writing systems、lexical aggregates、multi-writing-system examples、Unicode search、autosave、soft delete／Undo、英文與台灣繁中 UI。

### v0.2 / Export

rngagi-corpus v0.3 九欄 CSV、versioned export profile、可編輯 XeLaTeX project、Overleaf-ready ZIP、optional local PDF、ICU4X sorting、reverse index 與 portable font presets。

目前 CSV 相容性由 bkuw 內的 golden fixture 與人工 contract review 保護；尚無跨 `bkuw`／`rngagi-corpus` repositories 的自動 contract test。

v0.2.2 加入 tag-gated GitHub Release pipeline：全部 CI jobs 通過後自動建立含 Windows x64 NSIS、macOS Apple Silicon DMG、checksums 與 generated notes 的 unsigned Draft Release，再由 maintainer 發布。

v0.2.3 加入 bkuw-managed portable font packs：固定官方來源與 SHA-256、app-private cache、匯出內含 fonts/licenses、TeX Gyre Termes fatal requirement，並將 IPA 固定為 Charis SIL。

### v0.3 / Dictionary ordering and LaTeX refresh

Project-defined alphabet、entry section override 與 opt-in manual drag ordering；workspace 與匯出辭典共用順序及小標。XeLaTeX template 改善行距、詞條註解、IPA 詞頭、例句與 optional direct root/base related entries。Export snapshot 使用 bulk loading，長時間工作移至 background executor 並顯示階段進度。

v0.3.1 新增 Chiron Sung HK／Chiron Hei HK managed portable fonts 與明體／黑體風格提示；詞表將 IPA 合併至主要表記同行，並依序呈現每個義項自己的詞性與簡釋。

### v0.4 / Sense photos and search refinement

工作區搜尋擴充至 sense gloss 與 definition，並以 migration 5 回填 Unicode-safe search keys。詞表摘要最多顯示兩列，更多義項改顯示總數；LaTeX 辭典標題使用深紅粗體。

Sense-level 相片接受 PNG／JPEG／WebP，在本機 Canvas 輕度縮圖後統一保存為 project-relative PNG。Migration 6 保存圖片 metadata 與 SHA-256；LaTeX／PDF profile 可選擇是否把相片加入來源資料夾、Overleaf ZIP 與成品。

v0.4.1 修正 React field-array UI key 覆蓋持久化 sense ID，導致相片上傳誤報找不到義項的問題；同時細分詞條、義項與相片不存在的雙語錯誤訊息。CI／release 流程改為 version tag 重用完全相同 commit SHA 的成功 `main` artifacts，不再重複測試與平台打包。

v0.4.3 修正 WebView CSP 阻擋 sense 相片預覽的問題，並在預覽失敗時顯示雙語錯誤；LaTeX／PDF 匯出將相片縮入 `1000×900px`，不透明圖使用品質 82 JPEG、透明圖保留 PNG，減少雙欄辭典 PDF 體積且不改寫 project-local PNG。另新增 Windows `Ctrl+-/=/0`、macOS `Cmd+-/=/0` 的持久化 app zoom。發布流程新增單一 version preparation command；一般 push CI 不產生安裝包，只有 version commit 的 exact-SHA CI 成功後才自動建置 NSIS／DMG，在兩個平台完成後建立 checksums 與 exact-SHA Draft Release，人工 Publish 時才 materialize tag，並可從失敗 run 重用既有 installer artifacts。

### v0.5 / CSV import, font onboarding and semantic ordering

新增 UTF-8 自由 CSV 建立新專案的全頁流程：delimiter detection、任意 writing systems、欄位 mapping、相鄰列分組／拆分／合併、排除錯誤列、rngagi-corpus v0.3 預填與 notes 還原。來源 hash 與完整設定綁定 preview token；Rust 在 staging project 的單一 SQLite transaction 寫入所有 aggregates，失敗不留下半成品。

Portable fonts 改為首次啟動集中檢查六套 packs、批次進度與下載失敗後的離線例外；Hant `Auto`／zh-TW analysis text 採昭源宋體。Entry ordering V2 可依第一個有值的語意類別分組，configured／legacy／未分類順序與工作區、LaTeX／PDF 共用。辭典輸出另加入單義項省略編號及例／譯分行。
