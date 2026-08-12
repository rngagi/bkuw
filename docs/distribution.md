# CI 安裝包與簽章

## GitHub Actions 產物

`.github/workflows/ci.yml` 會在每次 push 與 pull request 執行驗證。各平台通過 checks、tests 與 Tauri build 後，會在該次 Actions run 的 **Artifacts** 區域提供：

- `bkuw-macos-apple-silicon`：Apple Silicon 的 `.app` 與 `.dmg`。
- `bkuw-windows-x64`：Windows x64 的 NSIS installer。

Artifacts 保存 14 天。macOS Intel 不在支援與建置範圍內，也不會上傳任何使用者資料。

推送與 app version 完全一致的 `v*` tag（例如 `v0.2.2`）時，同一 workflow 會在 portable-template、Windows x64 與 macOS Apple Silicon jobs 全數成功後：

1. 驗證 tag、`package.json`、Cargo 與 Tauri version 一致。
2. 收集一個 NSIS `.exe` 與一個 Apple Silicon `.dmg`。
3. 產生 `SHA256SUMS.txt`。
4. 建立 Draft GitHub Release、上傳三個 assets，並以 `.github/release.yml` 產生 categorized changelog。
5. Draft 經人工確認 assets 與說明後才發布；任一必要 job 失敗都不會建立 Release。

Release job 只取得 `contents: write`，一般 CI jobs 不取得發布權限。這個流程不會自動簽章。

## 目前的信任狀態

沒有平台憑證時仍可在 GitHub Actions 產生安裝包，但 macOS Gatekeeper 與 Windows SmartScreen 可能顯示未受信任警告。憑證、密碼、API key 與 cloud signing credentials 必須只放在 GitHub Actions secrets，不可提交到 repository。

## macOS Apple Silicon

### DMG 輸出

可以直接輸出 DMG。Tauri 在 macOS runner 執行下列命令時會同時建立 `.app` 與 `.dmg`：

```bash
pnpm tauri build --target aarch64-apple-darwin --bundles app,dmg
```

目前 GitHub Actions 已使用這個設定，成功後會將 DMG 放進 `bkuw-macos-apple-silicon` artifact。只需要 DMG 時也可執行 `pnpm tauri build --bundles dmg`。詳細格式與視窗自訂方式見 [Tauri DMG 官方文件](https://v2.tauri.app/distribute/dmg/)。

### Unsigned build 顯示「已損毀」

正式解法仍是 Developer ID signing、notarization 與 stapling。對尚未簽章、但已確認下載自本 repository Actions artifact 且未遭竄改的測試版，可依序處理：

1. 先嘗試開啟一次，再到「系統設定 → 隱私權與安全性」使用「仍要打開／Open Anyway」。這是 [Apple 建議的 per-app override](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac)。
2. 若仍顯示 `bkuw.app is damaged and can't be opened`，先確認 quarantine attribute：

   ```bash
   xattr -l /Applications/bkuw.app
   ```

3. 只有在確認來源可信時，才針對這個 app 移除 `com.apple.quarantine`：

   ```bash
   sudo xattr -dr com.apple.quarantine /Applications/bkuw.app
   ```

   `-d` 會刪除指定 attribute，`-r` 會處理整個 app bundle。Apple Developer Forums 也記錄了同一種 [`xattr -r -d com.apple.quarantine` 用法](https://developer.apple.com/forums/thread/727651)。若檔案不在 `/Applications`，必須改成實際的 `.app` 路徑。

不要使用 `spctl --master-disable` 全域關閉 Gatekeeper。若來源不明、簽章檢查異常，或重新下載後仍失敗，應刪除該檔案，而不是移除 quarantine；Apple 說明這類訊息也可能代表 app 確實遭修改或損壞：[Safely open apps on your Mac](https://support.apple.com/102445)。

### 正式簽章

正式站外散布建議使用 Apple Developer Program 的 **Developer ID Application** certificate，並完成 notarization：

1. 從 Keychain 匯出含 private key 的 `.p12`，以 base64 保存為 `APPLE_CERTIFICATE` secret。
2. 將 `.p12` 密碼保存為 `APPLE_CERTIFICATE_PASSWORD`；必要時以 `APPLE_SIGNING_IDENTITY` 指定 identity。
3. Notarization 建議使用 App Store Connect API key，設定 `APPLE_API_ISSUER`、`APPLE_API_KEY` 與 `APPLE_API_KEY_PATH`；也可使用 `APPLE_ID`、app-specific `APPLE_PASSWORD` 與 `APPLE_TEAM_ID`。
4. 確認 Tauri build 的 signing、notarization 與 stapling 均成功後，才把產物當作正式發布版本。

官方步驟：[Tauri macOS code signing](https://v2.tauri.app/distribute/sign/macos/)。

## Windows x64

正式散布需要 code-signing certificate 或 cloud signing service。新專案建議採 Microsoft Azure Artifact Signing：

1. 建立 Artifact Signing account、certificate profile 與具有 signer 權限的 service principal。
2. 將 `AZURE_CLIENT_ID`、`AZURE_CLIENT_SECRET`、`AZURE_TENANT_ID` 保存為 GitHub secrets。
3. 在 Windows runner 安裝所需的 Azure CLI、.NET、SignTool 與 Artifact Signing CLI。
4. 依實際 endpoint、account name、profile name 設定 Tauri `bundle.windows.signCommand`，讓 Tauri 在打包時簽署 executable 與 installer。

也可使用 CA 發行的硬體或雲端保管憑證；具體 CI 接法必須依發證機構規則設定。官方步驟：[Tauri Windows code signing](https://v2.tauri.app/distribute/sign/windows/)。

## 啟用正式簽章前的驗收

- Fork 或 pull request 不得取得 production secrets。
- 簽章只在受保護 branch 或 version tag 的 trusted workflow 啟用。
- macOS 用 `codesign --verify`、`spctl` 與 notarization log 驗證。
- Windows 用 `Get-AuthenticodeSignature` 或 SignTool 驗證 signer、timestamp 與 chain。
- Unsigned Release 必須清楚顯示 SmartScreen／Gatekeeper 警告、checksums 與 quarantine 安全說明；production signing 完成後再移除警告。
