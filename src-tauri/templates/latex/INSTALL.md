# bkuw LaTeX installation / LaTeX 安裝說明

bkuw uses XeLaTeX. A full distribution takes several GB of download and disk space. Installation needs internet access; after setup, bkuw can compile offline with its downloaded managed fonts. No dictionary data is uploaded.
bkuw 使用 XeLaTeX。完整發行版需要數 GB 下載與磁碟空間；安裝時需要網路。安裝完成並下載 bkuw 必要字型後可離線編譯，不會上傳辭典資料。

## Install / 安裝

1. Prefer Export → Local PDF → Check requirements → Install LaTeX. bkuw downloads and checks the installer, then opens the official installation wizard. Keep the full installation selected. Approve any system authorization in the system dialog only.
2. Finish the wizard, return to bkuw and choose Check again. Opening the installer does not mean installation succeeded.
3. If download or installation fails, retry or use the official links below. Existing TeX installations are never removed by bkuw.

1. 優先使用「匯出 → 本機 PDF → 檢查準備狀態 → 安裝 LaTeX」。bkuw 下載、驗證後開啟官方安裝精靈，請保留完整安裝選項。系統授權僅在系統視窗操作。
2. 完成官方精靈後回到 bkuw，按「重新檢查」。開啟精靈不代表安裝成功。
3. 下載或安裝失敗可重試，或使用下方官網。bkuw 不會移除原有 TeX 環境。

Windows: use a writable installation path without non-ASCII characters as required by TeX Live. Choose the full scheme. The pinned TeX Live 2025 installer uses its matching final repository. See [Windows installation](https://tug.org/texlive/windows.html).
Windows：依 TeX Live 限制選擇可寫入且不含中文等非 ASCII 字元的安裝路徑，選完整安裝。本腳本固定使用 TeX Live 2025 與對應的 final repository。[官方 Windows 說明](https://tug.org/texlive/windows.html)。

macOS Apple Silicon: the full MacTeX 2026 installer may ask for administrator authorization. See [MacTeX](https://tug.org/mactex/).
macOS Apple Silicon：完整 MacTeX 2026 安裝器可能需要管理員授權。[MacTeX 官網](https://tug.org/mactex/)。

## Optional scripts / 選用腳本

Review the saved script before running it. Windows: open PowerShell in this folder and run `powershell -NoProfile -ExecutionPolicy RemoteSigned -File .\install-windows.ps1`. Follow your organization's script policy if execution is blocked. macOS: open Terminal in this folder and run `sh ./install-macos.sh`. Scripts download from fixed HTTPS locations, check a pinned SHA-256, and open the official installer. They do not bypass system security or collect passwords. Downloaded installers are kept in a temporary folder and can be removed after installation.
先閱讀腳本再執行。Windows：在此資料夾開啟 PowerShell，執行 `powershell -NoProfile -ExecutionPolicy RemoteSigned -File .\install-windows.ps1`；若受政策阻擋，請遵守組織的腳本政策。macOS：在此資料夾開啟終端機，執行 `sh ./install-macos.sh`。腳本只從固定 HTTPS 來源下載、驗證固定 SHA-256 並啟動官方精靈，不繞過系統安全措施，也不收集密碼。安裝器保留於暫存資料夾，安裝完成可刪除。

## Existing installation / 已有安裝

If packages are missing, use TeX Live Utility / tlmgr for TeX Live or MacTeX, or MiKTeX Console for MiKTeX, to install the reported packages. Do not install another distribution over an existing one. Then check again in bkuw. If kpsewhich is unavailable, repair the distribution's command-line tools. The minimal probe also checks transitive dependencies and managed fonts.
若缺套件，請用 TeX Live／MacTeX 的 TeX Live Utility 或 tlmgr，或 MiKTeX 的 MiKTeX Console 安裝列出的套件；不要直接覆蓋原有發行版。完成後在 bkuw 重新檢查。若缺少 kpsewhich，請修復發行版的命令列工具。最小編譯也會驗證間接相依套件及 bkuw 管理的字型。

- [TeX Live package manager](https://tug.org/texlive/tlmgr.html)
- [MiKTeX Console](https://miktex.org/howto/miktex-console)
- [Overleaf ZIP import / ZIP 匯入](https://docs.overleaf.com/managing-projects-and-files/uploading-a-project)
- [XeLaTeX compiler / 編譯器設定](https://docs.overleaf.com/getting-started/recompiling-your-project/selecting-a-tex-live-version-and-latex-compiler)
