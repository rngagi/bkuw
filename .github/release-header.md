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
