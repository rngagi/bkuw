# CI 安裝包與簽章

## GitHub Actions 產物

`.github/workflows/ci.yml` 會在每次 push 與 pull request 執行驗證。各平台通過 checks、tests 與 Tauri build 後，會在該次 Actions run 的 **Artifacts** 區域提供：

- `bkuw-macos-apple-silicon`：Apple Silicon 的 `.app` 與 `.dmg`。
- `bkuw-windows-x64`：Windows x64 的 NSIS installer。

Artifacts 保存 14 天。這些檔案不會自動建立公開 GitHub Release，也不會自動簽章或上傳使用者資料。macOS Intel 不在支援與建置範圍內。

## 目前的信任狀態

沒有平台憑證時仍可在 GitHub Actions 產生安裝包，但 macOS Gatekeeper 與 Windows SmartScreen 可能顯示未受信任警告。憑證、密碼、API key 與 cloud signing credentials 必須只放在 GitHub Actions secrets，不可提交到 repository。

## macOS Apple Silicon

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
- 先保留 unsigned CI artifacts 作內部測試；正式 release workflow 與公開發布另行審核。
