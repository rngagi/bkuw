# bkuw 匯出指南 / Export guide

## 逐步匯出

1. 選擇輸出：預設「本機 PDF」，也可選 Overleaf ZIP、LaTeX 原始碼或 corpus CSV。
2. 設定內容：選擇書寫系統、圖片、字型、相關詞條等；CSV 設定 POS 對應。
3. 檢查準備狀態：修正資料錯誤、下載必要字型。本機 PDF 另檢查 XeLaTeX、套件並進行測試編譯。
4. 確認並匯出：檢視筆數、警告與輸出內容，選擇目的地。取消位置選擇會留在此步驟。
5. 結果：查看檔案路徑；Overleaf 依畫面教學自行上傳 ZIP。返回設定保留輸入，但必須重新檢查。

## 本機 LaTeX 安裝

缺少編譯器時按「安裝 LaTeX」，bkuw 下載並驗證完整 TeX Live（Windows x64）或 MacTeX（macOS Apple Silicon），再開啟官方精靈。請預留數 GB 下載與磁碟空間，保留完整安裝選項。系統授權僅在官方精靈／系統視窗操作。開啟精靈不代表安裝完成；完成後返回 App 按「重新檢查」。下載可取消與重試。若已關閉安裝精靈，可解除等待狀態後再下載。

已有環境但缺套件時，用該發行版的管理工具修復，不直接覆蓋。詳細步驟與手動腳本用法見 [雙語安裝指南](../src-tauri/templates/latex/INSTALL.md)。可在匯出準備步驟按「另存安裝腳本與說明」。安裝完成、必要字型已下載後可離線編譯；ZIP 本身不需要本機 TeX。

## Overleaf

1. [匯入 ZIP](https://docs.overleaf.com/managing-projects-and-files/uploading-a-project)：選 New Project → Upload Project，上傳 `*-overleaf.zip`。
2. [選擇 XeLaTeX](https://docs.overleaf.com/getting-started/recompiling-your-project/selecting-a-tex-live-version-and-latex-compiler)。
3. [設定主文件](https://docs.overleaf.com/getting-started/recompiling-your-project/the-main-document)：選根目錄的 `main.tex`。
4. [編譯](https://docs.overleaf.com/getting-started/recompiling-your-project)：按 Recompile 並處理顯示的錯誤。
5. [下載 PDF](https://docs.overleaf.com/managing-projects-and-files/downloading-a-project)。

Overleaf 需要網路與帳號；bkuw 不會自動上傳資料。ZIP 自帶字型、授權與選用圖片。若本機編譯失敗或逾時，來源與 `diagnostic.log` 仍保留；PDF 未產生時不會標示完成。

## Step-by-step export

1. Choose Local PDF (default), Overleaf ZIP, LaTeX sources, or corpus CSV.
2. Configure writing systems, photos, fonts and related entries; map POS for CSV.
3. Check requirements: resolve data issues and download required fonts. Local PDF also validates XeLaTeX, packages and a minimal compilation.
4. Confirm the counts, warnings and output, then choose a destination. Cancelling the destination keeps this step open.
5. Review the result paths. For Overleaf, follow the official links above to upload the ZIP, select XeLaTeX and root `main.tex`, recompile and download the PDF.

Back preserves settings and requires a fresh check. Local installer assistance downloads and verifies a full Windows TeX Live or macOS Apple Silicon MacTeX installer, then opens the official wizard. Keep the full installation selected and authorize only in the system UI. Allow several GB of download and disk space. Installer handoff is not success: finish installation and choose Check again. Download can be cancelled and retried. Repair existing installations with their own package manager. Save standalone scripts and the [bilingual installation guide](../src-tauri/templates/latex/INSTALL.md) from the requirements step if needed.

Once TeX and managed fonts are available, local compilation works offline. Overleaf requires an account and internet access; bkuw never uploads data automatically. Compilation failures preserve sources and diagnostic logs.

## 字型與輸出 / Fonts and output

字型由 bkuw 以固定來源、SHA-256 與 manifest 驗證後存入專用 cache，不需安裝至作業系統。TeX Gyre Termes 為必要字型，IPA 固定 Charis SIL，Hant Auto 與 zh-TW 分析文字使用昭源宋體。來源與 ZIP 包含實際使用的字型與授權。相片輸出為適合排版的衍生檔，專案原始 PNG 不變。CSV 維持 rngagi-corpus v0.3 九欄契約。

bkuw caches verified managed fonts without installing them into the OS. TeX Gyre Termes is mandatory, IPA uses Charis SIL, and Hant Auto / zh-TW analysis text use Chiron Sung HK. Sources and ZIP include used fonts and licenses. Photo derivatives are sized for print without changing project PNGs. Corpus CSV retains the rngagi-corpus v0.3 nine-column contract.
