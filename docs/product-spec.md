# bkuw 產品規格

## 產品目標

`bkuw` 是供單一語言學研究者離線蒐集與管理詞彙資料的桌面應用程式，優先服務 Windows x64 與 macOS Apple Silicon；不支援 macOS Intel。核心工作是大量、快速、可靠地輸入 dictionary-oriented lexical data，而不是進行完整 corpus annotation。

產品名稱在 UI、文件、視窗標題與產出檔案中一律寫作小寫 `bkuw`。

## 核心原則

- Local-first：不需帳號、雲端、遠端資料庫或網路。
- Lexical-entry centered：entry 是抽象詞彙項目，表記另依 writing system 保存。
- Unicode-native：支援 IPA、combining marks、中文、泰文、藏文、台灣原住民族語言及其他 Unicode scripts。
- Keyboard-efficient：建立、搜尋與儲存等高頻操作提供快捷鍵與可預測 tab order。Windows 使用 `Ctrl+-`／`Ctrl+=` 縮放、`Ctrl+0` 重設；macOS 使用對應的 `Cmd` 組合，讓不同 DPI／系統縮放環境可調整整個 app 顯示比例。
- Export-friendly：內部模型不綁定輸出格式；目前提供 versioned corpus CSV 與通用 XeLaTeX/PDF 輸出。
- Data integrity first：複合更新使用 transaction，schema 使用 migrations，升級前備份。

## Project 與 writing systems

一個 project 約對應一個語言或一份田野資料集。使用者可以定義任意數量的 writing systems，例如 Traditional Chinese、Pinyin、IPA，或 Tibetan、Wylie、IPA。

每個 project 必須指定一個 primary display writing system，可選擇一個不同的 secondary display writing system。列表不假設 native orthography 必然是 primary。

Writing system 可設定名稱、類型、script code、language tag、順序與顯示字型。類型包含 orthography、romanization、transliteration、phonemic、phonetic、other。

Script code 明確採 ISO 15924 四字母 Title Case 代碼，UI 驗證格式並提供 Unicode 官方查詢頁的系統瀏覽器連結。Project 另可指定 `zh-TW` 或 `en` analysis language；舊專案保持未設定，直到使用者需要匯出。

建立新 project 後立即開啟 writing-system onboarding：先說明 primary form 與實際例子；script code、BCP 47 language tag 與 font family 收在有逐欄說明的進階區域。Project 的 language code 明確採選填三字母 ISO 639-3，UI 提供官方 registry 的系統瀏覽器連結。已存在同名 `.bkuw` 路徑時必須以 modal 說明，不得覆寫。

Project settings 亦管理可重用的 POS 與語意類別選項。Sense editor 只從下拉選單選取，避免每個 sense 重複輸入 metadata。

## Lexical entries

Entry 可包含：

- 依 project writing systems 自動產生的 forms；每個 writing system 一個一般輸入欄位，避免重複新增相同表記。
- 任意數量的 ordered senses；POS 只保存在 sense。
- 每個 sense 下任意數量的 ordered examples。
- 每個 sense 可加入多張相片；接受 PNG、JPEG、WebP，過大來源在裝置上輕度縮圖後統一保存為 project-local PNG。Editor 必須顯示相片預覽，讀取或內容驗證失敗時顯示明確的本地化錯誤，不得永久停在載入狀態。
- root/base relations，可連到另一 entry；尚未建立 target 時可填「未連結詞形」，target 日後移除時也保留這段可讀 label。
- entry-level notes。

新增 example 時先建立 primary writing-system form；其餘 form 依尚未使用的 project writing systems 逐一加入，同一 example 不重複同一 writing system。Example 另有單一分析語言的 translation 與 notes。

Phonemic text 儲存時不包含 delimiter、顯示時加 `/…/`；phonetic text 顯示時加 `[…]`。使用者輸入原文不被改寫。

## 義項與例句音檔

每個 sense 與 example 可添加多個音檔，依添加順序排列；音檔不綁定 writing system，entry 層級沒有音檔。

從電腦選取 WAV、MP3、M4A/AAC、FLAC、OGG/Opus、AIFF、WebM 的常見音訊檔，使用隨附工具離線轉成 WebM／Opus、64 kbps VBR、單聲道、48 kHz。支援 PCM、MP3、AAC、ALAC、FLAC、Vorbis、Opus；空檔、損毀、無法解碼或含多個音軌的來源拒絕匯入。每個來源最多 256 MiB、30 分鐘，轉檔最多 5 分鐘；容許最多 50 ms 的編碼延遲。專案只保存 WebM 與 NFC 來源檔名，不修改電腦上的來源檔，不做降噪、音量正規化或靜音裁切。上傳提示只顯示接受的來源格式與限制，不顯示內部儲存參數。

可一次選取多檔，逐檔顯示進度與錯誤，成功的音檔即使其他檔案失敗也會保留。每筆顯示檔名、長度、大小，提供播放／暫停、拖曳進度及刪除。同時只播放一筆，離開詞條或專案時停止。播放失敗有英文與台灣繁中訊息。編輯義項／例句不會刪除音檔；刪除其擁有者時清理附件，詞條軟刪除則保留音檔供還原。

義項與例句提供程式內錄音；詞條本身不提供音檔。使用者按錄音後才請求麥克風權限，同時只允許一段錄音並停止其他播放。停止後可試聽、重新錄音、取消或儲存；儲存前僅留在記憶體，儲存時由 Rust 驗證並轉成相同 WebM／Opus 附件。錄音最多 30 分鐘、暫存最多 64 MiB；取消或離開畫面會釋放麥克風與暫存。保存失敗保留試聽內容供重試。沒有剪輯、音檔匯出或 corpus CSV contract 變更。專案只儲存及播放 WebM，不提供舊 MP3 或暫時 WAV 播放相容層。

## 核心使用流程

使用者必須能夠：

1. 建立、從 CSV 匯入，或開啟 `.bkuw` project。
2. 設定 project 資訊與 writing systems。
3. 在 two-pane workspace 搜尋、建立及選取 entries。
4. 編輯 forms、senses、sense-level POS、examples 與 root/base relations。
5. 為 sense 加入或移除相片，關閉並重開 project 後仍可顯示。
6. 以 autosave 或 `Ctrl/Cmd+S` 原子地保存完整 entry。
7. 軟刪除 entry 並立即 Undo。
8. 關閉並重新開啟 project，確認資料完全保存。
9. 在 English 與台灣繁中介面間即時切換。

所有 text inputs 與 textareas 關閉作業系統 autocorrect、autocapitalize 與 spellcheck，避免 macOS 等平台改寫語料。新增 entry 後 focus primary form；新增 example 後立即出現 primary form。

有效變更 debounce 後自動保存，toolbar 以 inline `尚未儲存 → 儲存中 → 已儲存` 呈現，不使用成功彈窗。中文、日文等 IME 正在組字時必須暫停 debounce save 與任何會 reset 表單的同步，等 `compositionend` 後再保存。Autosave 成功後不得 reset 或重建 entry form aggregate；使用者必須留在同一個 input／textarea，維持 focus、selection 與游標位置，才能不間斷繼續輸入。Entry transaction 成功後立即顯示已儲存；背景更新 entry list 不得延遲狀態。失敗時保留 dirty draft，顯示本地化 error code 的具體階段，並可展開 backend detail。切換 entry、關閉 project 或關閉 window 前必須等待 pending save 完成。

刪除 entry 前顯示二次確認；確認後 soft delete 並提供 immediate Undo。

## 搜尋

工作區搜尋 lexical entry forms，以及 sense 的簡釋與定義；不搜尋 POS、語意類別或 examples。搜尋為 Unicode-aware substring match，並使用衍生 search key 做 case-folding 與 diacritic folding；因此 `guo` 必須能找到表記或義項中的顯示值 `guò`。原始文字以 NFC 保存，搜尋處理不得改寫顯示資料。

目前搜尋涵蓋表記、gloss 與 definition；regex search 尚未提供。

## 從 CSV 建立專案

起始頁提供全頁式 CSV 流程：選檔與分隔符、專案與 writing systems、欄位 mapping、分組與驗證、目的地與結果。來源限 UTF-8／UTF-8 BOM，必須有非空白且不重複的標題列；自動判斷 comma、tab、semicolon，使用者可覆寫。React 不直接讀檔，inspection、解析與驗證都由 Rust command 完成。

使用者可先建立任意 writing systems 並指定 primary、type、script code 與 language tag，再把每個來源欄對應到 entry forms／notes、sense gloss／definition／POS／語意類別、各 writing system 的單一 example form、translation／notes、root fallback 或 Ignore。每個非 Ignore target 最多使用一次，且必須恰有一個 primary entry-form mapping。每個來源列建立一個 sense；example 相關欄全空時不建立 example。

預設將相鄰且 primary form 相同的列建議為同一 entry，使用者可拆分、合併相鄰群組或排除列。同組的 entry forms、entry notes 或 roots 不一致時阻擋匯入，不得取第一值。POS 與語意類別依來源首次出現順序建立 project options；已知 corpus POS code 同步建立 export mapping。Root cell 依使用者指定的單一分隔符（預設 `;`）拆成多筆未連結 fallback，不猜測同形詞 target。

CSV 錯誤必須在 wizard 內以目前 UI 語言直接說明可修正的原因。檔案檢查需指出無標題列、空白或重複標題名稱、非 UTF-8 byte 位置，以及欄數不符的實際來源列、預期欄數與實際欄數；preview issue DTO 同時帶來源列與欄索引，UI 以來源欄名顯示重複 mapping、重複 target、空白 primary form，以及詞形／詞條註記／詞根的個別分組衝突。Rust parser 的原始診斷不得成為唯一的使用者提示。

rngagi-corpus v0.3 的九欄順序會精確辨識並預填 mapping；`notes` 中的 `entry_notes`、`sense_definition`、`semantic_domain`、`example_notes` 會還原，未知內容附加至該列 sense definition。正式建立前重新讀取來源並以 SHA-256 綁定 preview token。Rust 產生 UUID、保存 NFC，在 staging project 的單一 SQLite transaction 寫入所有 aggregates；失敗移除 staging，不留下半成品 `.bkuw`。v0.5.0 只建立新專案，不匯入既有專案，也不支援 Big5、無標題列、單列多義項、多 examples、base relation 或自動 root linking。

## 詞條排序與小標

Project 自動排序可選擇以 writing system 或語意類別分組。Writing-system source 以一行一個元素定義字母表；`ng`、`ch` 等 multigraph 會視為單一排序元素。未定義字母表時，依 writing system language tag 使用 Unicode／ICU collation。語意類別 source 取每個 entry 第一個有值的 sense：依 Project Settings 選項順序排列，未列入設定的 legacy 值接續，未分類置底；類別內仍沿用指定 writing system、自訂 alphabet 或 ICU collation。自動排序同時產生 entry-list 與匯出辭典共用的小標。

以 writing system 自動排序時，每個 entry 可選擇自動小標或覆寫成 project alphabet 的其他小標。變更前必須二次確認並說明：只改變工作區與匯出辭典中的分組，不改寫表記、搜尋內容或該小標內的自然排序；因此 `ngungu` 可移入 `N` 小標，並仍以完整表記在 `N` 內自動排序。以語意類別分組時保留既有 override 資料但停用；完全自訂 manual layout 行為不變。

使用者明確確認後才可啟用完全自訂排序。專用介面可拖拉 entries 與 headings、建立或移除 headings，也可從目前自動排序匯入 headings 或從無 headings 開始。新 entry 暫時放在自動對應的小標末端並標示「尚未確認」，直到 layout 再次儲存。切回自動排序需確認；既有手動 layout 保留但不生效。完全自訂模式啟用時，entry-level 小標覆寫停用。

啟用完全自訂排序後，主工作區 header 持續提供「排列自訂順序」入口，不必先重新進入 Settings。若專案已是 manual mode 但尚未保存 layout，管理介面載入所有 live entries、說明尚未啟用的狀態，並允許使用者排列後首次保存完成復原。

## UX 與視覺

- 主 workspace 使用穩定、可調整寬度的 two-pane layout：左側搜尋與 entry list，右側 entry editor。
- 長表單與列表分別捲動，避免 modal-heavy workflow。
- entry list 顯示共用排序小標、primary form、同行 pronunciation、最多兩列 sense-level POS＋簡釋，以及手動 layout 中尚未確認的新詞條狀態；更多義項以總數摘要，不撐高列表項目。
- 使用 shadcn/ui 慣例、Radix primitives、Tailwind CSS、Lucide icons。
- 使用 system UI font；lexical forms 可依 writing system 選擇字型。
- primary accent 從 muted dark red `#b32b2b` 起始，僅作 semantic token。
- 禁止 gradients、glassmorphism、oversized cards、decorative shadows 與無必要動畫。唯一動畫例外是啟動畫面的 bkuw 字樣：使用隨附的 Pacifico 字型，尊重 `prefers-reduced-motion`，且不等待字型背景檢查才顯示專案入口。
- 不以紅色作為狀態的唯一線索；所有互動需具備 keyboard focus 與 accessible label。

## 發佈公開辭典網站

Header 提供「發佈網站」精靈；首次是設定並發佈，已有成功 deployment 時沿用同一組 Cloudflare 資源更新。精靈以新手可完成的 checklist 說明網站公開性、Workers Free plan、R2 checkout／免費額度與可能費用，並連到 Cloudflare 官方註冊、帳號與 R2 頁面。bkuw 只在使用者按下發佈時上傳，不背景同步。v1 固定使用 `workers.dev`，不處理自訂網域、網站刪除、回復、排程或 OAuth。

使用者輸入 32 字元 Account ID，經預填頁面建立只含 Workers Scripts Edit 與 Workers R2 Storage Edit 的 API Token。Token 驗證成功後存入 macOS Keychain 或 Windows Credential Manager，不寫入 `.bkuw`、SQLite、log 或 React state；credential store 不可用時只保留至程式關閉並提示使用者。中斷連線只刪除本機 Token，不刪除公開資源。

網站設定包含必填標題、選填 meta description、固定介面語言 `zh-TW`／`en`、選填的安全 Markdown info，以及 entry notes、example notes、root/base relations 與公開 writing systems。Primary writing system 必須公開，其餘系統可個別關閉。Markdown 支援標題、段落、粗斜體、清單、inline code 與 http(s)／mailto／頁內／相對連結；raw HTML 僅作文字顯示，不執行不安全 URL。精靈在同一頁說明 Worker 名稱、帳號子網域及完整公開 URL；帳號已有子網域時由 Cloudflare 讀取，不以 Worker 名稱代填。首次成功發佈前可明確套用新的帳號子網域，介面會警告這項設定會改變同帳號所有 Worker 網址。Worker 名稱在首次成功前可改，R2 bucket 固定衍生為 `<worker-name>-media`，首次成功後一起鎖定。

檢查前 flush autosave，Rust 建立不可變 publication snapshot，顯示詞項、義項、例句與媒體數量、預計網址、上傳／保留／待刪除媒體及 bytes。發佈時固定列出驗證、準備資源、上傳媒體、上傳網站、部署、線上驗證、清理與完成，個別標示尚未開始、執行中、已完成或失敗位置；進度條顯示目前階段的檔案數或 bytes。主要詞形、媒體路徑與 SHA-256、Markdown link、writing-system selection 及 25 MiB corpus JSON 上限都在發佈前阻擋。正式網站為桌面雙欄、手機列表／細項介面，支援 Unicode case／diacritic folding、hash deep-link、單一音檔播放、圖片 lazy loading、keyboard focus、200% 文字縮放、reduced motion、選填 info，以及跟隨系統後可保存的亮暗切換。公開詞項與分段標題直接沿用專案目前的自動或完全自訂排序，不在瀏覽器重新排序。亮色按鈕顯示月亮，深色顯示太陽；右下固定連到 bkuw 網站的小膠囊。發佈成功頁以原生剪貼簿複製網址，並用系統瀏覽器開啟正規化後的 `workers.dev/` 網址。

每個 project 使用一個 Worker 與一個專用 R2 bucket。網站程式與 `data/corpus.json` 使用 Workers Static Assets，圖片及 WebM 以 SHA-256 content-addressed key 放在 R2，Worker 以同網域提供 GET／HEAD／Range。更新只傳遠端缺少或大小不同的媒體；新 corpus digest、首頁與至少一個媒體線上驗證成功後才清理不再使用的 `media/` objects。bucket marker 與 Worker version annotation 不符時停止，不覆寫其他資源；清理失敗不讓已上線網站回報失敗，會保存 pending 狀態供下次重試。

## 語言支援

App UI 必須完整支援 `en` 與 `zh-TW`。首次啟動依 OS locale 決定，不支援時 fallback 至 English。使用者可在 app settings 即時切換並持久保存，不需重啟。

所有 user-facing strings、validation、errors、empty states、confirmations 皆使用 translation keys。使用者輸入的 lexical data 與 examples 不做自動翻譯。

## 匯出流程

Header 的 Export wizard 依「選擇輸出、設定內容、檢查準備狀態、確認並匯出、結果」逐步操作。預設本機 PDF，可主動選擇 Overleaf ZIP、LaTeX 原始碼或 corpus CSV。返回上一步保留設定；返回編輯會清除舊 preview，重新檢查後才能繼續。Preview 前必須 flush autosave；preview token 綁定當下 project snapshot，資料變動後不得以舊 token 匯出。Blocking error 會禁止輸出，warning 會說明無法表示或被省略的資料，並可導覽到相關 entry。

Preview、LaTeX/ZIP 產生與 XeLaTeX 編譯不得凍結 webview。等待期間顯示目前階段的本地化 indeterminate progress；PDF 明確說明 XeLaTeX 在背景執行且最長可能接近 120 秒。只有選擇 LaTeX／PDF 時才檢查 portable fonts，只有選擇 PDF 時才偵測 XeLaTeX，避免開啟一般 CSV wizard 時進行無關 I/O。

Corpus CSV 固定輸出 rngagi-corpus v0.3 的九欄簡版，每個未刪除 entry 的每個 sense 一列；analysis language 必須是 `zh-TW`。`gloss_en` 在單一 analysis-language 模型下留空。完整映射與 known loss 見 `docs/corpus-csv-contract.md`。

LaTeX 匯出包含可編輯來源資料夾與不含 PDF／aux／log 的 Overleaf-ready ZIP。辭典詞條順序與小標直接使用 project ordering；不另設會互相矛盾的 export collation。通用 template 使用 `fontspec`、雙欄、1.34 倍行距、懸掛縮排、頁眉、橢圓例句標記、欄內小標與 Rust 產生的 reverse index，不依賴日文專用套件或 makeindex。詞條 notes 以 `[註]` 標示並緊接詞頭下方、義項之前；恰有一個義項時省略 `1`，多義項完整編號。例句與翻譯分成兩行，zh-TW 使用「例／譯」、English analysis language 使用 `Ex.`／`Tr.`；空翻譯不產生翻譯行，例句註記留在同一 block 的 muted continuation。使用者可決定是否逐義項顯示語意類別；專案以語意類別自動分組時一律隱藏逐義項標示，避免與小標重複。中文 metadata 使用全形符號，例如「語意類別：」與 `；`。Profile 選定的 pronunciation（包含 IPA）只顯示在主要詞頭右側，不再以 `IPA: …` 或其他表記重複列出；headword 與 pronunciation 不得選用同一 writing system。所有 user text 完整 TeX escape。

LaTeX profile 可選擇不顯示關聯詞，或顯示 root、base、兩者。對每個 target entry 只收集直接指向它的 incoming live relations，最多一層、不遞迴，同一 source entry 去重；摘要顯示 headword、optional pronunciation 與第一個 sense gloss，並連回完整詞條。

LaTeX profile 另可選擇是否包含義項相片。開啟時，preview 必須驗證每張 project-local PNG 的路徑與 SHA-256；render 將圖片等比例縮入 `1000×900px` 且不放大，不透明圖以品質 82 JPEG、含透明像素的圖以 PNG 加入來源資料夾與 Overleaf ZIP，再於相應 sense 下以欄寬內、保持比例的方式排版。衍生圖只存在匯出結果，不得改寫專案內保存的 PNG；關閉時不得讀取或匯出相片。Corpus CSV 不表示相片。

bkuw 自行管理 portable font packs，不依賴 OS 已安裝字型，也不把字型安裝進系統。App 啟動時在背景檢查六套 catalog packs，不得擋住主畫面 wordmark 動畫或 project onboarding；語言選單旁固定顯示字型管理圖示，缺少或 invalid 時以狀態點提示，只有使用者開啟時才顯示獨立設定頁。設定頁可批次下載並查看每套 bytes／驗證／完成狀態。已完成套件不重抓；下載失敗後才可選擇離線使用，且不標記完成，下次啟動繼續提醒。Project Settings 與 Export wizard 都保留管理／重試入口。Rust 必須以固定 catalog URL 下載，經 staging、SHA-256 與 pack manifest 驗證後才寫入 app-private cache。匯出資料夾與 ZIP 必須包含實際需要的字型及授權檔。TeX Gyre Termes 是所有 LaTeX/PDF 匯出的必要 base；缺少或 invalid 時屬 fatal validation error。Phonemic／phonetic writing systems 固定使用 Charis SIL。Hant `Auto` 與 zh-TW analysis text 使用昭源宋體；UI 顯示「昭源宋體／昭源黑體」，但既有 pack ID、檔名與 upstream 官方名稱維持相容。其他 scripts 使用一般 Noto Serif fallback。現階段不提供 Thai／Tibetan 專用 managed font packs 或 presets。

左側詞表的主要表記與第一個可用 pronunciation form 顯示在同一行；若 pronunciation writing system 同時是 secondary，不得再顯示一次。下方依 sense order 顯示簡釋，每一列保留該 sense 自己的詞性與 gloss 配對；不同義項的詞性不得彙整成一個無法對應的清單。

本機 PDF 在準備步驟驗證 XeLaTeX 可執行、同一環境的 kpsewhich 可找到模板所需套件，再用 managed fonts 進行最小編譯。必要套件由模板推導；錯誤、缺字型與逾時會阻擋前進。CSV 不做字型／TeX 檢查；LaTeX 與 Overleaf 只需資料與字型驗證。PDF 只在本機偵測到 XeLaTeX 時產生。bkuw 在隔離 build directory 中以 `-no-shell-escape` 執行兩次並限制 120 秒；失敗保留來源與 diagnostic log，並在錯誤區顯示可選取、可複製的完整 log 路徑。若檢查後引擎消失，仍保存來源與 ZIP，但 UI 明確標示未產生 PDF。Overleaf 自選流程與此備援均提供官方 ZIP 匯入、XeLaTeX 設定、main.tex 主文件、編譯與 PDF 下載教學，不自動上傳 lexical data。

關閉主視窗時必須先完成有效草稿的 autosave 並釋放 project lock，之後程式才結束；Windows 與 macOS 的標準關窗操作皆須可用。

### LaTeX 安裝協助

缺少 XeLaTeX 時，使用者可按「安裝 LaTeX」。bkuw 從固定 HTTPS 來源串流下載完整 MacTeX 2026（macOS Apple Silicon）或 TeX Live 2025 final installer（Windows x64），驗證內建 SHA-256 後開啟官方精靈。Windows 使用同版本 final repository，請保留 full scheme；完整安裝需數 GB 網路與磁碟空間。系統授權交給官方精靈處理，App 不收集密碼。

下載有進度、取消與失敗重試。開啟精靈只表示等待安裝；回到 App 或手動按「重新檢查」才確認是否可編譯。若官方精靈取消或關閉，可選「安裝精靈已關閉，允許重新下載」再試。已有 TeX 時提供套件管理修復說明，不覆蓋安裝。可另外儲存 PowerShell／shell 腳本與雙語安裝說明，採相同固定 URL 與 SHA-256 驗證。安裝與字型下載完成後，本機編譯保持離線可用。

## 後續候選

候選工作、優先順序與尚待完成的人工驗收集中在 [執行清單](../plan.md)，不在產品規格重複維護。

## 明確排除

目前不包含 accounts、authentication、cloud sync、team collaboration、permissions、server backend、匯入既有專案、Big5 CSV、mobile、AI transcription、ASR、ELAN-style timeline、waveform segmentation、Git syncing、code signing、notarization、auto-update，或未經使用者主動觸發的自動上傳 lexical data。Publish corpus website 所需的 Cloudflare authorization、部署與 media upload 是已列入 backlog 的唯一雲端例外。受信任的 `main` version commit 通過 exact-SHA CI 後可自動建立 unsigned Draft GitHub Release；正式發布前須人工確認安裝包、checksums 與警告內容。
