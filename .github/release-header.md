## Downloads / 下載

- Windows x64: download the NSIS `setup.exe` installer.
- macOS Apple Silicon: download the `.dmg` image. macOS Intel is not supported.
- Verify downloads with `SHA256SUMS.txt` when needed.

## Unsigned build notice / 未簽署版本提醒

These installers are not yet code-signed or notarized. Windows SmartScreen and macOS Gatekeeper may show a warning.

這些安裝包尚未進行程式碼簽章或 Apple notarization，Windows SmartScreen 與 macOS Gatekeeper 可能顯示警告。

If a trusted macOS download reports that `bkuw.app` is damaged, first try **System Settings → Privacy & Security → Open Anyway**. If necessary, verify the download checksum and then run:

```bash
sudo xattr -dr com.apple.quarantine /Applications/bkuw.app
```

Only remove quarantine after confirming that the app came from this repository and its checksum matches.
