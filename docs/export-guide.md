# bkuw 匯出指南 / Export guide

## 台灣繁中

1. 在專案設定確認 analysis language；rngagi-corpus CSV 必須選「繁體中文（台灣）」。
2. 從 header 開啟「匯出」，選 CSV、LaTeX 或 PDF。
3. 設定 POS mapping、書寫系統、逆向索引、關聯詞與 portable font presets。LaTeX 的詞條順序及小標使用「專案設定 → 詞條排序」。
4. 按「預覽」。bkuw 會先完成 autosave；阻擋錯誤必須修正，warnings 則說明格式無法表示的資料。
5. 選擇目的地。既有 CSV 需要二次確認，寫入採 temporary sibling file 與 atomic replacement。

LaTeX 輸出資料夾包含 `main.tex`、`entries.tex`、`reverse-index.tex`、`.latexmkrc`、本 README 內容，以及本機成功時的 `dictionary.pdf`。同層 `*-overleaf.zip` 包含來源檔、需要的 portable fonts 與授權檔，不包含 PDF、log 或 aux files。可選的關聯詞只顯示直接 incoming root／base links 一層，並連回完整詞條。

找不到 XeLaTeX 時，來源與 ZIP 仍會正常建立。到 Overleaf 建立 Upload Project、上傳 ZIP，並將 compiler 設為 XeLaTeX。bkuw 不會自動上傳詞彙資料。

字型不需要也不應手動安裝到作業系統。Export wizard 會列出 TeX Gyre Termes、Charis SIL、Noto Serif、Noto Serif CJK TC、Chiron Sung HK（昭源宋體）與 Chiron Hei HK（昭源黑體）的狀態；按「下載並重試」後，bkuw 從官方固定版本下載、驗證 SHA-256 並保存於專用 cache。匯出設定中，每個非 IPA 書寫系統都能選字型；Chiron Sung HK 會標示為明體／宋體風格，Chiron Hei HK 會標示為黑體／無襯線風格，兩者都採香港字形慣例。TeX Gyre Termes 是必要 base，缺少或損毀時 LaTeX/PDF preview 會直接阻擋；IPA（phonemic／phonetic）固定使用 Charis SIL。現階段不提供 Thai／Tibetan 專用 managed font packs。輸出的 `fonts/` 目錄與 Overleaf ZIP 會自帶需要的字型及授權檔，因此不依賴 Windows、macOS 或 Overleaf 原本安裝的 fonts。首次下載完成後，cache 可離線重用。

編譯失敗或超過 120 秒時，來源專案與 `diagnostic.log` 會保留。錯誤畫面會顯示完整的診斷紀錄位置；可複製該路徑，並在 Windows 檔案總管或 macOS Finder 前往該檔案。錯誤畫面也會區分 validation、stale preview、filesystem、compile 與 timeout；修正後重新 preview 再匯出。

## English

1. Confirm the project analysis language. The rngagi-corpus CSV requires Taiwan Traditional Chinese (`zh-TW`).
2. Open Export from the app header and choose CSV, LaTeX, or PDF.
3. Configure POS mappings, writing systems, reverse index, related entries, and portable font presets. LaTeX entry order and headings come from Project Settings → Entry ordering.
4. Select Preview. bkuw flushes autosave first. Blocking errors must be fixed; warnings identify data the target format cannot represent.
5. Choose a destination. Replacing an existing CSV requires confirmation and uses a temporary sibling plus atomic replacement.

The LaTeX folder contains `main.tex`, `entries.tex`, `reverse-index.tex`, `.latexmkrc`, a bilingual README, and `dictionary.pdf` when local compilation succeeds. The sibling `*-overleaf.zip` contains source files, required portable fonts, and licenses—never PDF, logs, or auxiliary files. Optional related entries include one level of direct incoming root/base links and link back to each full entry.

If XeLaTeX is unavailable, source and ZIP generation still succeeds. In Overleaf, create an Upload Project, upload the ZIP, and select XeLaTeX. bkuw never uploads lexical data automatically.

Do not install these fonts into the operating system. The Export wizard lists TeX Gyre Termes, Charis SIL, Noto Serif, Noto Serif CJK TC, Chiron Sung HK, and Chiron Hei HK. “Download and retry” fetches a fixed official version, verifies its SHA-256 digest, and stores it in bkuw's private cache. Each non-IPA writing system can select its export font; Chiron Sung HK is identified as a Ming/Song style, while Chiron Hei HK is identified as a Hei/sans-serif style. Both follow Hong Kong glyph conventions. TeX Gyre Termes is mandatory, so a missing or invalid pack blocks LaTeX/PDF preview. IPA (phonemic/phonetic) always uses Charis SIL. Dedicated Thai and Tibetan managed font packs are not currently offered. Exported projects and Overleaf ZIPs carry their required fonts and license files under `fonts/`, independent of fonts installed on Windows, macOS, or Overleaf. Once downloaded, the cache can be reused offline.

On a compile failure or 120-second timeout, bkuw preserves the source project and `diagnostic.log`. The error panel shows the full diagnostic-log path so it can be copied and opened from Windows File Explorer or macOS Finder. Correct the reported problem, create a fresh preview, and export again.
