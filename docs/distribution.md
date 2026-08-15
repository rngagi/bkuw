# CI 安裝包與簽章

## GitHub Actions 產物

`.github/workflows/ci.yml` 在 `main` push 與 pull request 執行 checks、tests、release-mode desktop E2E 與 no-bundle app build。一般 CI 不建立 NSIS／DMG，也不呼叫 `actions/upload-artifact`；因此每次 push 不會留下安裝包 artifacts。macOS Intel 不在支援與建置範圍內，也不會上傳任何使用者資料。

準備新版本時只使用單一命令，不手動逐檔改版本或 push tag：

```bash
pnpm release:prepare -- 0.4.3
git diff --check
git commit -am "chore: prepare v0.4.3"
git push origin main
```

`release:prepare` 要求乾淨的 `main` worktree，確認新版本是遞增的 stable semantic version，並一起更新 `package.json`、`Cargo.toml`、`Cargo.lock` 與 Tauri config。版本一致性也可獨立以 `pnpm release:check -- 0.4.3` 驗證。

version commit 的 `main` CI 成功後，`.github/workflows/release.yml` 自動：

1. 確認來源是本 repository 的 trusted `main` push，且 portable-template、Windows x64 與 macOS Apple Silicon jobs 全數成功。
2. 讀取 exact CI commit 的一致版本，並和 Git history 中前一個 package version 比較；只有版本確實遞增且對應 tag 尚不存在時才繼續。因此 version commit 後同一批 push 即使還有 workflow／文件修正，也不會漏掉 release candidate；一般未升版 commit 會正常結束，不打包。
3. 在 release workflow 的 macOS Apple Silicon runner 建置 `.app`／`.dmg`，並在 Windows x64 runner 建置 NSIS installer。
4. 僅在這次 release run 上傳 `bkuw-macos-apple-silicon` 與 `bkuw-windows-x64` 暫存 artifacts，保存 7 天供失敗恢復。
5. Final job 收集一個 `.dmg` 與一個 `.exe`、產生並重驗 `SHA256SUMS.txt`；兩個平台都成功後，才建立以 exact commit 為 target、含三個 assets 與 categorized changelog 的 Draft GitHub Release。Draft 階段尚未 materialize Git tag，人工 Publish 時 GitHub 才建立 tag。
6. Draft 經人工確認 assets 與說明後才發布；來源不可信、CI 失敗、build 失敗、版本不一致或 checksum 錯誤都不會建立 Release。

一般 CI jobs 不取得發布權限；只有 release final job 取得 `contents: write`。發布順序為「逐功能完成驗證並 commit → `release:prepare` → push version commit 到 `main` → GitHub 自動建立 Draft → 人工 Publish」。Release 不重跑整套 tests，只利用 Rust cache 執行平台 packaging。這個流程不會自動簽章。

一般失敗使用 GitHub 的 **Re-run failed jobs**，已成功的 installer jobs 不需重跑。若必須先修正 workflow，可在 Actions 手動執行 `release` recovery，輸入 version、通過 CI 的 exact target SHA，以及仍在 7 天保存期內的失敗 release run ID；workflow 會驗證 run path 與 SHA，再下載既有 artifacts，跳過兩個平台 packaging，重新建立或更新同一 Draft。`publish-draft` 只允許移動尚未 materialize tag 的 Draft target，或更新 tag 已指向相同 commit 的 Draft；已公開 Release 不可覆寫。

Desktop E2E 使用 release-mode app，而非未最佳化的 debug app。這可使包含 portable fonts 與 ZIP 的 LaTeX export 保持在 WebdriverIO Tauri direct-eval 的時間限制內；Rust integration tests 仍負責完整 export failure／rollback coverage。

## 目前的信任狀態

沒有平台憑證時仍可在 GitHub Actions 產生安裝包，但 macOS Gatekeeper 與 Windows SmartScreen 可能顯示未受信任警告。憑證、密碼、API key 與 cloud signing credentials 必須只放在 GitHub Actions secrets，不可提交到 repository。

## macOS Apple Silicon

### DMG 輸出

可以直接輸出 DMG。Tauri 在 macOS runner 執行下列命令時會同時建立 `.app` 與 `.dmg`：

```bash
pnpm tauri build --target aarch64-apple-darwin --bundles app,dmg
```

Release workflow 使用這個設定，成功後會將 DMG 放進該次 release run 的 `bkuw-macos-apple-silicon` artifact，再上傳至 Draft Release。只需要 DMG 時也可執行 `pnpm tauri build --bundles dmg`。詳細格式與視窗自訂方式見 [Tauri DMG 官方文件](https://v2.tauri.app/distribute/dmg/)。

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
- 簽章只在受保護 `main` version commit 的 trusted release workflow 啟用。
- macOS 用 `codesign --verify`、`spctl` 與 notarization log 驗證。
- Windows 用 `Get-AuthenticodeSignature` 或 SignTool 驗證 signer、timestamp 與 chain。
- Unsigned Release 必須清楚顯示 SmartScreen／Gatekeeper 警告、checksums 與 quarantine 安全說明；production signing 完成後再移除警告。
