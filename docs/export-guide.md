# bkuw 匯出指南 / Export guide

## 台灣繁中

1. 在專案設定確認 analysis language；rngagi-corpus CSV 必須選「繁體中文（台灣）」。
2. 從 header 開啟「匯出」，選 CSV、LaTeX 或 PDF。
3. 設定 POS mapping、書寫系統、排序、分節、逆向索引與 portable font presets。
4. 按「預覽」。bkuw 會先完成 autosave；阻擋錯誤必須修正，warnings 則說明格式無法表示的資料。
5. 選擇目的地。既有 CSV 需要二次確認，寫入採 temporary sibling file 與 atomic replacement。

LaTeX 輸出資料夾包含 `main.tex`、`entries.tex`、`reverse-index.tex`、`.latexmkrc`、本 README 內容，以及本機成功時的 `dictionary.pdf`。同層 `*-overleaf.zip` 只包含五個 source files，不包含 PDF、log 或 aux files。

找不到 XeLaTeX 時，來源與 ZIP 仍會正常建立。到 Overleaf 建立 Upload Project、上傳 ZIP，並將 compiler 設為 XeLaTeX。bkuw 不會自動上傳詞彙資料。預設 portable fonts 是 Charis SIL／Noto Serif、Noto Serif CJK TC、Noto Serif Tibetan 與 Noto Serif Thai；字型未安裝時可能出現 missing glyph，請安裝對應字型或在 export profile 更換 preset。

編譯失敗或超過 120 秒時，來源專案與 `diagnostic.log` 會保留。錯誤畫面會區分 validation、stale preview、filesystem、compile 與 timeout；修正後重新 preview 再匯出。

## English

1. Confirm the project analysis language. The rngagi-corpus CSV requires Taiwan Traditional Chinese (`zh-TW`).
2. Open Export from the app header and choose CSV, LaTeX, or PDF.
3. Configure POS mappings, writing systems, collation, sections, reverse index, and portable font presets.
4. Select Preview. bkuw flushes autosave first. Blocking errors must be fixed; warnings identify data the target format cannot represent.
5. Choose a destination. Replacing an existing CSV requires confirmation and uses a temporary sibling plus atomic replacement.

The LaTeX folder contains `main.tex`, `entries.tex`, `reverse-index.tex`, `.latexmkrc`, a bilingual README, and `dictionary.pdf` when local compilation succeeds. The sibling `*-overleaf.zip` contains only the five source files—never PDF, logs, or auxiliary files.

If XeLaTeX is unavailable, source and ZIP generation still succeeds. In Overleaf, create an Upload Project, upload the ZIP, and select XeLaTeX. bkuw never uploads lexical data automatically. Default portable fonts are Charis SIL/Noto Serif, Noto Serif CJK TC, Noto Serif Tibetan, and Noto Serif Thai. Install the appropriate font or change the profile preset if glyphs are missing.

On a compile failure or 120-second timeout, bkuw preserves the source project and `diagnostic.log`. Correct the reported problem, create a fresh preview, and export again.
