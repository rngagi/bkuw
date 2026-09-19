# bkuw XeLaTeX export / bkuw XeLaTeX 匯出

This folder is an editable dictionary project generated locally by bkuw.
本資料夾是 bkuw 在本機產生、可繼續編輯的辭典專案。

## Overleaf

1. Choose **New Project → Upload Project** in Overleaf.
2. Upload the sibling `*-overleaf.zip` file.
3. Use XeLaTeX if Overleaf does not select it automatically.

1. 在 Overleaf 選擇 **New Project → Upload Project**。
2. 上傳同層的 `*-overleaf.zip`。
3. 若未自動選取，請將 compiler 設為 XeLaTeX。

No lexical data is uploaded by bkuw. / bkuw 不會自行上傳任何詞彙資料。

Required fonts and their license notices are included under `fonts/`. The project does not rely on fonts installed in the operating system or Overleaf.
所需字型與授權聲明均已放在 `fonts/`；本專案不依賴作業系統或 Overleaf 原本安裝的字型。

Entry order and headings follow the bkuw project ordering settings. Optional related-entry lists contain direct links only, one level deep.
詞條順序與小標依 bkuw 專案排序設定；選用的關聯詞清單只包含一層直接連結。

## Local compilation / 本機編譯

Prefer the bkuw Local PDF wizard. It checks XeLaTeX, packages and managed fonts before export. For manual setup, see [INSTALL.md](INSTALL.md). To compile this exported folder manually, run `xelatex -no-shell-escape -interaction=nonstopmode -halt-on-error main.tex` twice from this directory.
優先使用 bkuw 本機 PDF 匯出精靈；會先檢查 XeLaTeX、套件與字型。手動安裝請參考 [INSTALL.md](INSTALL.md)。若直接編譯此資料夾，請在此目錄執行兩次 `xelatex -no-shell-escape -interaction=nonstopmode -halt-on-error main.tex`。

## Official Overleaf guides / Overleaf 官方教學

1. [Upload ZIP / 匯入 ZIP](https://docs.overleaf.com/managing-projects-and-files/uploading-a-project).
2. [Select XeLaTeX / 選擇 XeLaTeX](https://docs.overleaf.com/getting-started/recompiling-your-project/selecting-a-tex-live-version-and-latex-compiler).
3. [Set root main.tex / 設定根目錄 main.tex](https://docs.overleaf.com/getting-started/recompiling-your-project/the-main-document).
4. [Recompile / 編譯](https://docs.overleaf.com/getting-started/recompiling-your-project).
5. [Download PDF / 下載 PDF](https://docs.overleaf.com/managing-projects-and-files/downloading-a-project).
