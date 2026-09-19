# Changelog

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
