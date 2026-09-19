# bkuw 0.6.0

## Changelog / 變更記錄

### Features / 新功能

- Export is now a five-step wizard: choose output, configure content, check requirements, confirm export, and review the result.
- Local PDF export checks XeLaTeX, required TeX packages, managed fonts, and a minimal compilation before proceeding.
- Missing LaTeX distributions can be downloaded, SHA-256 verified, and handed off to the official Windows x64 TeX Live or macOS Apple Silicon MacTeX installer.
- Saved PowerShell and shell installation scripts include bilingual setup instructions.
- Overleaf guidance now covers ZIP import, selecting XeLaTeX, setting `main.tex`, recompiling, and downloading the PDF.
- The application icon was updated in commit `4b38efb`.

### Fixes and maintenance / 修正與維護

- LaTeX checks distinguish missing engines, missing packages, failed probes, and timeouts, with preserved diagnostic logs.
- Returning to export settings preserves edits while requiring a fresh preview and requirement check.
- Failed PDF compilation keeps the LaTeX sources and Overleaf ZIP without claiming PDF completion.

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
