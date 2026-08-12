# bkuw 產品規格

## 產品目標

`bkuw` 是供單一語言學研究者離線蒐集與管理詞彙資料的桌面應用程式，優先服務 Windows x64 與 macOS Apple Silicon；不支援 macOS Intel。核心工作是大量、快速、可靠地輸入 dictionary-oriented lexical data，而不是進行完整 corpus annotation。

產品名稱在 UI、文件、視窗標題與產出檔案中一律寫作小寫 `bkuw`。

## 核心原則

- Local-first：不需帳號、雲端、遠端資料庫或網路。
- Lexical-entry centered：entry 是抽象詞彙項目，表記另依 writing system 保存。
- Unicode-native：支援 IPA、combining marks、中文、泰文、藏文、台灣原住民族語言及其他 Unicode scripts。
- Keyboard-efficient：建立、搜尋與儲存等高頻操作提供快捷鍵與可預測 tab order。
- Export-friendly：內部模型不綁定輸出格式；v0.2 提供 versioned corpus CSV 與通用 XeLaTeX/PDF 輸出。
- Data integrity first：複合更新使用 transaction，schema 使用 migrations，升級前備份。

## Project 與 writing systems

一個 project 約對應一個語言或一份田野資料集。使用者可以定義任意數量的 writing systems，例如 Traditional Chinese、Pinyin、IPA，或 Tibetan、Wylie、IPA。

每個 project 必須指定一個 primary display writing system，可選擇一個不同的 secondary display writing system。列表不假設 native orthography 必然是 primary。

Writing system 可設定名稱、類型、script code、language tag、順序與顯示字型。Milestone 1 類型包含 orthography、romanization、transliteration、phonemic、phonetic、other。

Script code 明確採 ISO 15924 四字母 Title Case 代碼，UI 驗證格式並提供 Unicode 官方查詢頁的系統瀏覽器連結。Project 另可指定 `zh-TW` 或 `en` analysis language；舊專案保持未設定，直到使用者需要匯出。

建立新 project 後立即開啟 writing-system onboarding：先說明 primary form 與實際例子；script code、BCP 47 language tag 與 font family 收在有逐欄說明的進階區域。Project 的 language code 明確採選填三字母 ISO 639-3，UI 提供官方 registry 的系統瀏覽器連結。已存在同名 `.bkuw` 路徑時必須以 modal 說明，不得覆寫。

Project settings 亦管理可重用的 POS 與 semantic-domain 選項。Sense editor 只從下拉選單選取，避免每個 sense 重複輸入 metadata。

## Lexical entries

Entry 可包含：

- 依 project writing systems 自動產生的 forms；每個 writing system 一個一般輸入欄位，避免重複新增相同表記。
- 任意數量的 ordered senses；POS 只保存在 sense。
- 每個 sense 下任意數量的 ordered examples。
- root/base relations，可連到另一 entry；尚未建立 target 時可填「未連結詞形」，target 日後移除時也保留這段可讀 label。
- entry-level notes。

新增 example 時先建立 primary writing-system form；其餘 form 依尚未使用的 project writing systems 逐一加入，同一 example 不重複同一 writing system。Example 另有單一分析語言的 translation 與 notes。多分析語言 translations 留待後續版本。

Phonemic text 儲存時不包含 delimiter、顯示時加 `/…/`；phonetic text 顯示時加 `[…]`。使用者輸入原文不被改寫。

## Milestone 1 使用流程

使用者必須能夠：

1. 建立或開啟 `.bkuw` project。
2. 設定 project 資訊與 writing systems。
3. 在 two-pane workspace 搜尋、建立及選取 entries。
4. 編輯 forms、senses、sense-level POS、examples 與 root/base relations。
5. 以 autosave 或 `Ctrl/Cmd+S` 原子地保存完整 entry。
6. 軟刪除 entry 並立即 Undo。
7. 關閉並重新開啟 project，確認資料完全保存。
8. 在 English 與台灣繁中介面間即時切換。

所有 text inputs 與 textareas 關閉作業系統 autocorrect、autocapitalize 與 spellcheck，避免 macOS 等平台改寫語料。新增 entry 後 focus primary form；新增 example 後立即出現 primary form。

有效變更 debounce 後自動保存，toolbar 以 inline `尚未儲存 → 儲存中 → 已儲存` 呈現，不使用成功彈窗。中文、日文等 IME 正在組字時必須暫停 debounce save 與任何會 reset 表單的同步，等 `compositionend` 後再保存，避免注音候選字或輸入焦點跳開。Entry transaction 成功後立即顯示已儲存；背景更新 entry list 不得延遲狀態。失敗時保留 dirty draft，顯示本地化 error code 的具體階段，並可展開 backend detail。切換 entry、關閉 project 或關閉 window 前必須等待 pending save 完成。

刪除 entry 前顯示二次確認；確認後 soft delete 並提供 immediate Undo。

## 搜尋

Milestone 1 搜尋 lexical entry forms，不搜尋 examples。搜尋為 Unicode-aware substring match，並使用衍生 search key 做 case-folding 與 diacritic folding；因此 `guo` 必須能找到顯示值 `guò`。原始文字以 NFC 保存，搜尋處理不得改寫顯示資料。

進階 FTS、語言特定 collation、fuzzy search、example 全文搜尋與自訂 sort key 留待後續版本。

## UX 與視覺

- 主 workspace 使用穩定、可調整寬度的 two-pane layout：左側搜尋與 entry list，右側 entry editor。
- 長表單與列表分別捲動，避免 modal-heavy workflow。
- entry list 顯示 primary form、optional secondary form 與 senses 彙整的 POS。
- 使用 shadcn/ui 慣例、Radix primitives、Tailwind CSS、Lucide icons。
- 使用 system UI font；lexical forms 可依 writing system 選擇字型。
- primary accent 從 muted dark red `#b32b2b` 起始，僅作 semantic token。
- 禁止 gradients、glassmorphism、oversized cards、decorative shadows 與無必要動畫。
- 不以紅色作為狀態的唯一線索；所有互動需具備 keyboard focus 與 accessible label。

## 語言支援

App UI 必須完整支援 `en` 與 `zh-TW`。首次啟動依 OS locale 決定，不支援時 fallback 至 English。使用者可在 app settings 即時切換並持久保存，不需重啟。

所有 user-facing strings、validation、errors、empty states、confirmations 皆使用 translation keys。使用者輸入的 lexical data 與 examples 不做自動翻譯。

## v0.2 匯出流程

Header 的 Export wizard 依「格式、profile、validation preview、目的地、結果」操作。Preview 前必須 flush autosave；preview token 綁定當下 project snapshot，資料變動後不得以舊 token 匯出。Blocking error 會禁止輸出，warning 會說明無法表示或被省略的資料，並可導覽到相關 entry。

Corpus CSV 固定輸出 rngagi-corpus v0.3 的九欄簡版，每個未刪除 entry 的每個 sense 一列；analysis language 必須是 `zh-TW`。`gloss_en` 在單一 analysis-language 模型下留空。完整映射與 known loss 見 `docs/corpus-csv-contract.md`。

LaTeX 匯出包含可編輯來源資料夾與不含 PDF／aux／log 的 Overleaf-ready ZIP。通用 template 使用 `fontspec`、雙欄、懸掛縮排、頁眉、例句標記與 Rust 產生的 reverse index，不依賴日文專用套件或 makeindex。所有 user text 完整 TeX escape。

PDF 只在本機偵測到 XeLaTeX 時產生。bkuw 在隔離 build directory 中以 `-no-shell-escape` 執行兩次並限制 120 秒；失敗保留來源與 diagnostic log。找不到引擎仍是成功的 LaTeX 匯出，UI 提供 Overleaf 上傳步驟，但不自動上傳 lexical data。

## 後續里程碑

- v0.2：rngagi-corpus v0.3 CSV、XeLaTeX project、Overleaf ZIP 與 optional local PDF。
- 後續：audio import/playback、optional recording。
- 後續：IPA helper、tags、filters、duplicate detection、backup manager、進階搜尋與多 analysis-language translations。

## 明確排除

v0.2 不包含 accounts、authentication、cloud sync、team collaboration、permissions、server backend、audio、CSV import、mobile、AI transcription、ASR、ELAN-style timeline、waveform segmentation、Git syncing、code signing、notarization、auto-update、自動上傳或公開 release。
