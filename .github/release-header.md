# bkuw 0.6.3

## 變更記錄 / Changelog

### 中文

- 義項與例句加入程式內錄音，停止後可試聽、重錄、取消或儲存；匯入與錄音統一轉為單聲道 WebM／Opus 64 kbps VBR、48 kHz，新增 WebM 來源支援。移除舊 MP3 儲存相容層，不建立 WAV 播放副本。音檔仍不包含在 CSV／LaTeX／PDF 匯出中。
- 更新啟動畫面與隨附 Pacifico 字型的字樣呈現；支援減少動態效果偏好。字型檢查改為背景執行，語言選單旁提供狀態與管理入口，不再阻擋建立或開啟專案。

### English

- Added in-app recording for senses and examples, with preview, re-record, cancel and save. Imports and recordings now use mono WebM/Opus at 64 kbps VBR / 48 kHz, including WebM source support. Removed legacy MP3 storage compatibility; no WAV playback copies are created. Audio remains excluded from CSV, LaTeX and PDF exports.
- Refreshed the startup screen with a bundled Pacifico wordmark and reduced-motion support. Font checks now run in the background, with status and management beside the language selector, without blocking project creation or opening.

# bkuw 0.6.2

## 變更記錄 / Changelog

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

# bkuw 0.6.1

## 變更記錄 / Changelog

### 中文

- 修正桌面應用程式與 Windows/macOS 安裝程式仍使用舊 icon 的問題。
- 重新產生 Tauri 的 PNG、ICO、ICNS、Windows、Android 與 iOS icon 資產。

### English

- Fixed desktop app and Windows/macOS installer bundles still using the old icon.
- Regenerated the Tauri PNG, ICO, ICNS, Windows, Android, and iOS icon assets.

## 0.6.0 變更記錄 / Changelog

### 新功能

- 匯出流程改為五個步驟：選擇輸出、設定內容、檢查準備狀態、確認並匯出、查看結果。
- 本機 PDF 匯出會檢查 XeLaTeX、必要 TeX 套件、bkuw 管理的字型與最小編譯結果。
- 缺少 LaTeX 編譯器時，可下載並驗證 Windows x64 TeX Live 或 macOS Apple Silicon MacTeX，再開啟官方安裝精靈。
- 提供 Windows PowerShell、macOS shell 安裝腳本與雙語安裝說明。
- Overleaf 流程涵蓋 ZIP 匯入、選擇 XeLaTeX、設定 `main.tex`、重新編譯與下載 PDF。
- 應用程式 icon 已更新，來源 commit 為 `4b38efb`。

### 修正與維護

- LaTeX 環境檢查會區分缺少編譯器、缺少套件、編譯失敗與逾時，並保留診斷紀錄。
- 返回匯出設定後會保留輸入內容，但要求重新預覽與檢查，避免使用過期結果。
- PDF 編譯失敗時會保留 LaTeX 原始碼與 Overleaf ZIP，不會誤顯示為 PDF 匯出完成。

## 下載

- Windows x64：下載 NSIS `setup.exe` 安裝程式。
- macOS Apple Silicon：下載 `.dmg` 映像檔；不支援 macOS Intel。
- 如需確認檔案完整性，請使用 `SHA256SUMS.txt` 驗證下載檔案。

## 未簽署版本提醒

這些安裝包尚未進行程式碼簽章或 Apple notarization，Windows SmartScreen 與 macOS Gatekeeper 可能顯示警告。

如果可信任的 macOS 下載檔顯示 `bkuw.app` 已損毀，請先嘗試「系統設定 → 隱私權與安全性 → 強制打開」。必要時先驗證下載檔案的 checksum，再執行：

```bash
sudo xattr -dr com.apple.quarantine /Applications/bkuw.app
```

只有在確認 App 來自本 repository 且 checksum 相符後，才移除 quarantine 標記。
