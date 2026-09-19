# Changelog

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
