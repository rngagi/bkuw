# bkuw

本版本的變更由 GitHub 依兩個版本之間的 commit 自動列在本段之後。完整版本歷史見 [`CHANGELOG.md`](https://github.com/rngagi/bkuw/blob/main/CHANGELOG.md)。

## 下載

- Windows x64：下載 NSIS `setup.exe` 安裝程式。
- macOS Apple Silicon：下載 `.dmg` 映像檔；不支援 macOS Intel。
- 如需確認檔案完整性，請使用 `SHA256SUMS.txt` 驗證下載檔案。

## 未簽章安裝包

這些安裝包尚未進行程式碼簽章或 Apple notarization，Windows SmartScreen 與 macOS Gatekeeper 可能顯示警告。

如果可信任的 macOS 下載檔顯示 `bkuw.app` 已損毀，請先嘗試「系統設定 → 隱私權與安全性 → 強制打開」。必要時先驗證下載檔案的 checksum，再執行：

```bash
sudo xattr -dr com.apple.quarantine /Applications/bkuw.app
```

只有在確認 App 來自本 repository 且 checksum 相符後，才移除 quarantine 標記。Windows SmartScreen 也可能在未簽章版本顯示警告。

安裝與驗證說明見[發布文件](https://github.com/rngagi/bkuw/blob/main/docs/distribution.md)。
