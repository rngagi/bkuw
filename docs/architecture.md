# bkuw 架構與資料模型

## 系統形狀

React 負責 presentation、interaction、draft state 與 localization；Rust 負責 project lifecycle、CSV parsing/import、filesystem、validation、SQLite、migrations、backup、locking、font-pack supply chain 與 aggregate transactions。

```text
React UI
  ↓ typed adapter + Zod validation
Tauri commands
  ↓
Project/database module
  ↓
SQLite + project filesystem
```

Frontend 不可直接執行 SQL。所有 `invoke` 集中在 `src/lib/tauri.ts`，讓 command 名稱、DTO shape 與 error mapping 只有一個 seam。Project/database module 的外部 interface 提供 create/open/close project、settings、entry query/load/create/save/delete/restore 與 export snapshot；連線、SQL、normalization、backup 和 transaction 都留在 implementation 內。Export module 是 deep module：公開 preview/run/detect 行為，內部封裝 corpus flattening、ICU4X sorting、TeX rendering/escaping、ZIP、atomic write 與 XeLaTeX process。

## Project lifecycle

- 一次只允許一個 active project。
- 建立 project 時產生 `<name>.bkuw/project.sqlite` 與 `backups/`，遇到既有路徑不得覆寫。
- CSV 建立新 project 時先使用同一 parent 下的 hidden staging project；所有 writing systems、metadata 與 entry aggregates 在單一 SQLite transaction 寫入，成功關閉 connection／lock 後才 rename 成 `<name>.bkuw`，失敗移除 staging。
- 開啟時 canonicalize 路徑、驗證目錄與 database identity/schema，再取得 exclusive project lock。
- migration 前以 SQLite-consistent 方法建立 timestamped backup；migration 失敗時保留原資料並回報 stable error code。
- 關閉 project 時先 flush pending save，再關閉 connection 與釋放 lock。
- Tauri main window 僅有 open/save dialog、必要 core capability，以及 scope 嚴格限定為官方 ISO 639-3、Unicode ISO 15924、Overleaf project／官方匯入、編譯器、主文件、編譯、下載教學，以及 TeX Live／MacTeX／MiKTeX 官方說明 URL 的 opener permission；不開放 shell、HTTP 或 broad filesystem plugin。Project database 操作限制在 active canonical project；export 只操作使用者經 dialog 選定的目的地。

## Command interface

主要 commands：

```text
create_project(request) -> ProjectSnapshot
open_project(path) -> ProjectSnapshot
close_project() -> void
update_project_settings(request) -> ProjectSnapshot
query_entry_summaries(query) -> EntrySummary[]
load_entry(id) -> LexicalEntry
create_entry() -> LexicalEntry
save_entry(aggregate, expectedRevision) -> LexicalEntry
list_sense_images(senseId) -> SenseImage[]
attach_sense_image(request) -> SenseImageMutation
load_sense_image(imageId) -> SenseImageContent
remove_sense_image(request) -> SenseImageMutation
list_audio(owner: AudioOwner) -> AudioAttachment[]
import_audio(request: ImportAudioRequest) -> AudioMutation
load_audio(audioId) -> AudioContent
remove_audio(request: RemoveAudioRequest) -> AudioMutation
begin_audio_recording(request) -> string (project session token)
save_audio_recording(request) -> AudioMutation
delete_entry(id, expectedRevision) -> DeletedEntry
restore_entry(id) -> LexicalEntry
save_export_settings(settings) -> ExportSettingsV1
preview_export(kind) -> ExportPreview
export_project(request) -> ExportResult
detect_xelatex() -> TexEngineStatus
check_latex_environment() -> LatexEnvironment
install_latex(onProgress) -> void
cancel_latex_download() -> void
save_latex_install_guide(destination) -> string
list_font_packs() -> FontPackStatus[]
install_font_pack(packId) -> FontPackStatus
install_font_packs(packIds, progressChannel) -> FontPackStatus[]
inspect_csv(path, delimiter?) -> CsvInspection
preview_csv_import(request) -> CsvImportPreview
create_project_from_csv(request, previewToken) -> CsvImportResult
save_entry_sort_settings(settings) -> ProjectSnapshot
save_manual_sort_layout(layout) -> ProjectSnapshot
```

Errors 使用 `{ code, message, details? }`，其中 export 另穩定區分 `export_validation`、`export_stale`、`export_filesystem`、`latex_compile`、`latex_timeout`，字型管理另使用 `font_download`、`font_integrity`、`font_filesystem`、`font_unknown`。UI 顯示依 code 本地化的安全訊息；compile failure/timeout 的 detail 指向保留的 diagnostic log，frontend 只針對這兩個 code 將完整路徑顯示為可選取文字，不把其他內部 error details 外洩。

Main window 的 close request 由 React 攔截，先 flush entry autosave、關閉 active project session，再呼叫 Tauri `destroy()` 完成真正關窗。Capability 僅對 `main` window 額外授權 `core:window:allow-destroy`；這是 `core:default` 未包含、Windows 會強制檢查的必要權限。

App-level zoom shortcut controller 使用 Tauri WebView `setZoom`，只額外授權 `core:webview:allow-set-webview-zoom`。Windows `Ctrl` 與 macOS `Cmd` 搭配 `-`／`=` 依序使用 67%、80%、90%、100%、110%、125%、150% 的 bounded levels，`0` 回到 100%；同時處理 shifted `+` 與 numpad key variants。比例保存於 device-local `localStorage`，啟動時套用；IME composition、AltGr 與其他 app shortcuts 不被攔截。

`save_entry` 接收 forms、senses、examples、example forms 與 relations 的完整 aggregate，在單一 transaction 內以 replace-diff strategy 寫入。Sense rows 使用 upsert／delete diff，而不是全部刪除重建，避免一般 autosave cascade 掉仍存在 sense 的相片。`revision` 使用 optimistic concurrency 防止較舊 autosave 覆蓋新資料。

相片二進位不放進 entry aggregate。Frontend `imageCompression` adapter 依指定的 Canvas 流程解碼 PNG／JPEG／WebP，只有長邊超過 2560px 時等比例縮圖，再輸出 PNG；這是輕度尺寸處理，不承諾固定 byte 上限。Attach／remove command 會先 flush entry，使用同一 entry revision 做 optimistic concurrency。Rust 重新解碼 PNG、取得可信尺寸、計算 SHA-256，先寫 temporary sibling，再於 DB transaction 內更新 revision 與 metadata；load 只接受 DB 中由 active project 指向的固定 `media/images/<uuid>.png`。Frontend 驗證回傳內容的 PNG signature，使用 CSP 已允許的 `data:image/png;base64,...` 顯示，不需 `blob:` 或 filesystem capability；失敗以明確 preview error 結束 loading。刪除 sense 成功後清理失去 DB reference 的檔案。

## 義項與例句音檔

`database::audio` 負責本機轉檔、檔案界線、完整性與附件交易。`AudioOwner` 是 `{ kind: "sense" | "example", id }`；新增／移除要求 entry ID 與 expected revision，回傳更新後的 entry 和可選的附件 metadata。音訊 bytes 不進入 entry aggregate。透過唯一的 frontend adapter 呼叫；檔案選擇沿用 dialog 權限。

匯入前 flush autosave，Rust 驗證擁有者與 revision，取得當次 project session token，釋放 session mutex 後在 blocking task 轉檔。來源須為 regular file，限 256 MiB；複製至隔離暫存目錄後，以隨附的 FFprobe 檢查唯一音軌、codec 與最長 30 分鐘，再以 FFmpeg/libopus 轉成 64 kbps VBR／mono／48 kHz WebM。兩個程序共用 5 分鐘 deadline；逾時 kill 並 reap，損毀輸入或截斷輸出不提交。輸出以 FFprobe 再驗證，允許至多 50 ms 的 Opus 編碼延遲，最多 32 MiB。

完成後重新鎖定 session，比對 session token、擁有者及 revision。輸出先寫至 project-local temporary sibling 並 sync，再於 SQLite transaction 內新增 metadata、遞增 revision，搬至 `media/audio/<uuid>.webm`；提交失敗移除新檔。檔案 metadata 包含來源檔名、長度、大小、SHA-256、排序與建立時間。例句使用 upsert/delete diff，避免 autosave cascade 刪除音檔。

讀取只接受 UUID WebM project-relative 路徑，拒絕 media/audio 目錄及檔案 symlink，檢查檔案大小與 SHA-256。Frontend 收到 typed `audio/webm` base64 後建立 Blob URL；CSP 只新增 `media-src blob:`。播放延遲載入，每次播放請求停止前一筆，忽略過期回應，unmount 時 pause 並 revoke URL。刪除義項／例句後清理失去 reference 的音檔；entry soft delete 保留 bytes 供 Undo。音檔不參與 CSV／LaTeX／PDF 匯出。不提供舊 MP3 路徑或暫時 WAV 播放副本。

錄音由 `audioCapture.ts` 管理 MediaRecorder、單一麥克風與播放互斥、30 分鐘／64 MiB 上限及取消清理。按錄音先 flush 並取得 project session token，才呼叫 getUserMedia；MediaRecorder 使用 WebM／Opus，平台不支援時可使用 MP4 暫存，儲存結果一律為 WebM。無 duration 的 streaming container 在完整解碼後驗證長度。停止後以記憶體 Blob 試聽；儲存時重新 flush，傳送 token、owner、revision 與 bounded base64，由 Rust 在隔離暫存目錄驗證／轉檔，再經原有附件 transaction 提交。跨 project session 的結果拒絕儲存。macOS Info.plist 提供麥克風用途說明；未新增 shell、網路或廣域檔案權限。

固定 FFmpeg 8.0.1 與 Opus 1.6.1 來源 URL／SHA-256 由 `scripts/audio/prepare.sh` 管理，禁用網路與非必要 codecs，不啟用 GPL／nonfree，Windows 靜態連結工具 runtime。`pnpm audio:prepare` 首次由來源建置工具，開發／build 先檢查工具；Windows 開發需 MSYS2 MINGW64，`ensure.mjs` 以 `BKUW_MSYS2_LOCATION` 指定安裝目錄，預設 `C:/msys64`；CI／release 傳入 setup action 回傳的實際位置。建置以 shell 工具產生 manifest，不依賴 Python。安裝包包含 tools、binary SHA-256 manifest、licenses、完整來源 archives 與 build recipe；終端使用者不需安裝或下載轉檔工具。既有 CI 的平台 jobs 驗證真實轉檔與 WebView 播放，Linux fast-checks 不建置桌面音訊工具。

## SQLite schema

所有 IDs 使用 UUID，timestamps 使用 UTC RFC 3339。所有 connections 啟用 foreign keys、busy timeout，並使用適合單機桌面程式的 WAL mode。

核心 tables：

- `projects`：identity、name、ISO 639-3 language metadata、timestamps。
- `projects.analysis_language`：nullable `zh-TW`／`en`；舊專案 migration 後仍為 null。
- `export_settings`：project-owned versioned JSON profile；目前 schema version 為 1。
- `writing_systems`：project、name、type、script/language tags、display role、sort order、font。
- `metadata_options`：project-owned POS／語意類別 reusable values 與 sort order。
- `lexical_entries`：project、notes、optional section override、revision、timestamps、soft-delete timestamp。
- `entry_sort_settings`：project-owned versioned JSON；V2 保存 auto/manual mode、`writingSystem | semanticDomain` source、組內排序 writing system 與 ordered alphabet elements。
- `manual_sort_layouts`：project-owned versioned JSON；保存 headings 與 entry IDs 的線性 layout。
- `entry_forms`：entry、writing system、NFC text、derived search key、metadata、sort order。
- `senses`：entry、gloss、definition、POS、語意類別、sort order。
- `sense_images`：sense、project-relative PNG path、原始檔名、尺寸、byte size、SHA-256、sort order、created timestamp。
- `audio_attachments`：sense 或 example 擇一擁有、project-relative WebM path、來源檔名、長度、byte size、SHA-256、sort order、created timestamp。
- `examples`：sense、translation、notes、sort order。
- `example_forms`：example、writing system、NFC text、sort order。
- `entry_relations`：source、optional target、relation type、fallback text、notes、sort order。
- `schema_migrations`：已套用的 migration versions。

Display-role constraints 保證 primary 恰有一個、secondary 最多一個且兩者不同。Initial project setup 可在同一 transaction 建立 writing systems 與 primary role，避免中間無效狀態。

Owned children 使用 `ON DELETE CASCADE`。Relation target 被永久移除時使用 `SET NULL` 並保留 fallback label。被 entry forms 或 example forms 引用的 writing system 使用 `RESTRICT`。Relation 不得 self-reference，並須有 target 或非空 fallback。

## Unicode 與搜尋

- 顯示文字在 Rust 寫入前正規化為 NFC。
- `entry_forms.search_key` 與 `senses.search_key` 是可重建的衍生欄位：Unicode case fold、分解、移除 combining marks、再正規化。Sense key 只由 gloss＋definition 組成，不混入 POS 或語意類別。
- query 使用同一演算法，對 form 或 sense search key 做 substring matching；Chinese、Tibetan、Thai、IPA 等未折疊內容仍保留並可搜尋。
- 不假設 code point 等於 grapheme；character-level UI behavior 必須使用 grapheme-aware APIs。
- Example forms 保存同樣的正規化文字，但目前不納入 entry-list search。

## Autosave 與刪除

- 有效 draft 變更經 debounce 後保存；同一 entry 的 saves 排序執行。
- DOM composition events 是 autosave boundary：`compositionstart` 清除 debounce timer，active composition 期間不得送出 save 或用 backend snapshot reset form，`compositionend` 後才重新排程。
- Autosave success 不以 `reset(savedAggregate)` reconcile。Editor 只原地同步 Rust 管理的 `revision`／`updatedAt` 並更新 committed snapshot，避免 `useFieldArray` 重新產生 keys、重建巢狀 controls 與移走 focus／selection；真正切換或重新載入 entry 時才 reset aggregate。
- `Ctrl/Cmd+S`、entry/project 切換與 window close 會先 flush。
- Entry aggregate transaction 回傳成功時立即更新 inline live status，不等待非關鍵的 entry-list refresh；list refresh 在背景執行。Failure 保留 draft 與 dirty state，顯示 retryable localized error 與可展開 backend detail。
- Entry delete 先經 confirmation，之後設定 `deleted_at` 並從一般 query 排除；UI 提供 immediate Undo。完整 Trash manager 後續再做。

Frontend adapter 對 Rust unit response 接受 Tauri JSON `null`，再映射為 TypeScript `void`；這適用於 `close_project` 等 commands。Window close handler prevent default 後依序 flush、close session、destroy window，任一步失敗都保留視窗與 draft。

Entry forms 在 frontend 依 writing-system settings 自動補齊並固定排序；example 先建立 primary form，再允許加入尚未使用的 writing system。Phonemic／phonetic delimiter 是 presentation concern，不寫回 lexical text。Document-level input policy 透過既有及動態 controls 統一關閉 autocorrect、autocapitalize、autocomplete 與 spellcheck。

Migration 2 新增 `metadata_options`。Migration 3 新增 `projects.analysis_language` 與 `export_settings`。Migration 4 新增 entry section override、versioned sort settings 與 manual layout。Migration 5 新增並以 Rust Unicode folding 回填 `senses.search_key`。Migration 6 新增 `sense_images`，媒體檔則放在 project 的 `media/images/`。Migration 7 新增 `audio_attachments`，以互斥的 sense/example 外鍵及 cascade delete 維持擁有者，音檔位於 `media/audio/`。舊 schema 開啟時仍遵守先建立一致性 SQLite backup、再於 transaction 套用 migration 的規則。

Migration 8 新增 `publish_settings` 與 `publish_deployments`。前者保存 versioned 網站設定與固定 Cloudflare resource names；後者保存 Account ID、Worker、bucket、workers.dev subdomain、公開 URL、遠端版本、corpus digest、最後發佈時間與 cleanup state。API Token 不在 schema 內，複製 project 到另一台裝置時必須重新連線。

## Ordering module

Rust `ordering` module 是工作區與 LaTeX 匯出的集中排序 seam。`EntrySortSettingsV2` 增加 `source = writingSystem | semanticDomain`；V1 JSON 以 serde default 讀成 writing-system source，在下次保存時寫回 V2，SQLite row version 仍沿用 migration 4 contract，不需 schema migration。輸入為 live entry summaries、project sort settings、manual layout、language tag 與語意類別 options；輸出包含確定順序、section label 與 `manualOrderPending`。

LaTeX export profile 的 `includeSemanticDomains` 決定是否逐義項輸出語意類別，舊 profile 缺少此欄時預設為 `true`。React 在 automatic semantic-domain grouping 下將控制項顯示為強制關閉；Rust `export` module 也以相同條件強制抑制逐義項 metadata，避免只靠 UI 維持輸出不重複的 invariant。

Writing-system source 的自訂 alphabet 使用 longest-match tokenization，確保 `ng` 不被拆為 `n`＋`g`；未定義 alphabet 時使用 ICU4X collator。Section override 只替換 group key，full form sort key 不變。語意類別 source（內部值 `semanticDomain`）以 sense sort order 取得第一個非空值：configured options 先依設定順序，legacy values 接續依 label，空值最後；組內仍比較相同 writing-system sort key。此 source 不讀 section override，React editor 同時停用該 control。

Manual layout 把 heading 與 entry 當作同一線性序列。已刪除 entry 在讀取時忽略；layout 未收錄的新／恢復 entry 依自動規則插入對應 section 尾端並標示 pending。切回 auto 不刪除 layout。Frontend 只送出 typed settings/layout commands，不自行推導持久化順序。

## CSV import module

`csv_import` 是建立新 project 的 deep module。公開介面只有 inspection、preview 與 create；內部封裝 UTF-8／BOM 驗證、comma／tab／semicolon detection、header contract、mapping discriminated unions、row materialization、rngagi notes parser、group conflict rules、metadata collection、UUID、NFC、preview token 與 aggregate construction。React 只透過 `src/lib/tauri.ts` 選檔並傳送 typed DTO，不讀來源 bytes、SQLite 或 project filesystem。

Preview token 由來源 raw bytes 的 SHA-256 與完整 `CsvPreviewRequest` 序列化共同產生。Create 重新讀檔並重跑 mapping、分組、排除列與 validation；token 不同回傳 `stale_preview`。每個 included row 建立一個 sense，example fields 全空時不建立 example；相鄰相同 primary form 只是建議分組，explicit groups 必須相鄰、互斥並覆蓋全部 included rows。Entry-level forms／notes／roots 在同組有多個 distinct value 時回傳 blocking issue。

Create 階段才將 frontend-local writing-system IDs remap 成 Rust UUID，並為 entries、forms、senses、examples、relations 產生 UUID。POS／語意類別依 materialized source order 去重；known POS 同步寫入 corpus export mappings。Database module 接收完成的 aggregates，在 staging project 的同一 transaction 替換預設 writing system、寫入 project metadata/export settings 及全部 aggregates，然後移到正式路徑。

CSV inspection 將可修正的 parse failure 分成 stable error codes，並以 JSON `details` 傳遞 byte／row／expected／actual 等非本地化參數；React wizard 使用翻譯 key 組成完整訊息。Preview validation 的 `CsvPreviewIssue` 帶 `rowIndices`、`columnIndices` 與 optional stable target key，讓 UI 能從 inspection DTO 還原實際來源欄名與 writing-system target。分組衝突在 Rust 依 entry form、entry notes、roots 分別產生 issue，不合併成無法定位的通用錯誤。

`ManualSortItem` 的 Tauri JSON contract 固定使用 camelCase，尤其 entry variant 必須是 `entryId`；Rust 以 `rename_all_fields` 保證 tagged enum 的 struct fields 與 TypeScript schema 一致。Manual mode 若因舊版部分成功狀態而缺少 layout，workspace 仍提供直接管理入口，editor 載入所有 live entries 並在首次保存時建立 layout，無須手動修資料庫。

## Export architecture

`ProjectSession` 依 `ProjectSnapshot.entries` 的既定順序建立只含 live entries 的完整 `ExportSnapshot`，並附上每個 entry 的 section label、active project root 與 live sense-image metadata；LaTeX renderer 不再自行排序。完整 aggregate 以固定組數的 bulk queries 載入 forms、senses、examples、example forms、relations 與 image metadata，再於 Rust 組裝，避免資料量增加時出現巢狀 N+1 queries。Preview 以 snapshot + format 的 SHA-256 token 綁定資料；真正匯出前重新建立 snapshot，token 不同即回傳 `export_stale`。React 不讀 SQL 或 filesystem，所有 DTO 由 `src/types/domain.ts` 的 Zod schema 驗證。

Preview、font integrity scan、XeLaTeX detection 與 export 都透過 Tauri async command 將 blocking 工作移到 background executor。Project mutex 只持有到一致的 `ExportSnapshot` 建立完成，render、ZIP、font copy 與 XeLaTeX 執行期間不持鎖，因此不會阻塞 webview repaint，project close 也不必等待最長 120 秒的 PDF compile。Frontend adapter interface 維持單一 typed invoke seam。

CSV renderer 固定 rngagi-corpus v0.3 九欄。ICU4X 依 profile language tag 排 primary form，entry UUID 與 sense order 是 deterministic tie-breakers。Writer 使用 UTF-8、無 BOM、CRLF 及 RFC 4180 quoting。輸出先寫同層 temporary sibling；Unix 使用 replace rename，Windows 使用 `MoveFileExW` 的 replace/write-through flags，避免留下半成品。

LaTeX renderer 從零建立通用 XeLaTeX source，不複製 `docs/main.tex` 的授權巨集。所有 user text 經集中 escaping；writing-system font macros 使用純字母 control sequence 與 project-relative font paths。Pronunciation writing system 的 form 只傳入詞頭 macro 的右側參數，並從其他 forms metadata 排除。Export settings 的 Rust validation 保證 headword／pronunciation IDs 不同，frontend 同時過濾重複選項；舊 profile 若重複則 normalize 為未指定 pronunciation。Related-entry renderer 依 profile 選擇 root/base/both，掃描 export snapshot 中直接指向 target 的 relations；snapshot 已排除 soft-deleted entries，source 以 entry 為單位去重且不遞迴。Reverse index 由 Rust 排序並直接產生 `hyperlink`／`pageref`，不使用 makeindex。

`includeSenseImages` 預設為 false，以 serde default 相容舊 export profile。啟用時，preview 與 render 都只讀 `media/images/<uuid>.png` 並驗證 PNG signature 與 DB SHA-256。Render 在記憶體中以 Lanczos3 將來源等比例縮入 `1000×900px` 且不放大；實際不透明圖使用品質 82 JPEG，含有效透明像素的圖使用 best-compression PNG，再以對應的 `.jpg`／`.png` 路徑加入 source tree。LaTeX folder、Overleaf ZIP 與隔離 PDF build 共用同一組衍生 bytes，project-local PNG 不被改寫；template 使用 `graphicx` 限制欄寬與最大高度並保持比例。未啟用時不讀或打包媒體，CSV renderer 永遠不表示相片。

Font manager 是另一個 deep module。固定 catalog 包含 TeX Gyre Termes、Charis SIL、Noto Serif、Noto Serif CJK TC、Chiron Sung HK 與 Chiron Hei HK，並記錄 pack ID、上游固定 commit/release、HTTPS URL、archive members、逐檔與 archive SHA-256、版本、LaTeX faces 與授權檔。兩個 Chiron packs 使用上游 fixed tag 的 static OTF Regular／Bold 與 SIL OFL 1.1 授權；不從浮動 branch 下載。下載先進 app-local staging directory；只有 archive 與每個 extracted/downloaded file 全部通過雜湊驗證，才以 manifest 啟用 cache。cache 每次使用前依 manifest 重驗，損毀 pack 視為 invalid。React 不接觸網路或 filesystem，只能列出狀態與請求安裝；Rust HTTP client 只能使用 catalog 內建 URL。

App mount 後只在背景列出一次六套字型狀態，不作為 startup gate，也不延後 ProjectStart 的 wordmark 動畫或 project onboarding。語言選單旁的固定圖示是字型管理入口；缺少或 invalid 時顯示狀態點，setup page 只由使用者開啟。Batch install command 逐 pack 重用 verified completion、以 typed Tauri channel 回報 downloading bytes、verifying、installed／failed；失敗後 UI 才開放當次離線繼續，不保存「已完成」旗標。Hant `Auto` 與 zh-TW analysis font resolver 指向既有 `chiron-sung-hk` pack；upstream catalog identity 不改名。

TeX Gyre Termes 是所有 LaTeX/PDF export 的 mandatory base pack，缺少或 invalid 時 preview 產生 fatal blocking issue。分析語言與每個 writing system 依 profile/script 決定其他必要 packs；phonemic／phonetic 類型不接受 preset override，固定解析為 Charis SIL。需要的字型檔與相應 license 都放進 `fonts/<pack-id>/`，LaTeX folder 與 Overleaf ZIP 因此不依賴 OS font registry。

Entry list 的 read model 由 database 一次載入 forms，再以固定的一個 bulk query 載入 ordered sense summaries。每個 summary 保存自己的 POS 與 gloss，禁止先各自彙整後再嘗試配對。Pronunciation form 依 phonetic 優先、phonemic 次之選出，連同 writing-system ID 回傳；React 使用該系統的 `/…/`／`[…]` 顯示規則，並在它也是 secondary 時抑制重複行。

ZIP 打包 `main.tex`、`entries.tex`、`reverse-index.tex`、`.latexmkrc`、bilingual `README.md`／`INSTALL.md`、`fonts/` 下的必要 font/license files，以及 profile 啟用時的 `images/`，不含 PDF/log/aux。PDF runner 從 PATH、macOS TeX path 與 Windows 常見路徑找 XeLaTeX，把完整 sources tree 複製到 temporary build directory，兩次執行 `-no-shell-escape -interaction=nonstopmode -halt-on-error -file-line-error`，每次最多 120 秒。成功只複製 PDF；失敗/timeout 保留 source project 與 `diagnostic.log`。

CSV 的外部相容契約見 `docs/corpus-csv-contract.md`。目前沒有跨 repository 自動 contract test；`rngagi-corpus` 版本變更必須人工重驗與更新 golden fixture。

## Frontend structure

```text
src/
├── App.tsx
├── components/ui/
├── features/projects/
├── features/settings/
├── features/entries/
├── features/export/
├── features/publish/
├── features/fonts/
├── i18n/
├── lib/
└── types/
```

React Hook Form 管理 entry aggregate draft，Zod 負責 frontend validation。`App.tsx` 的 React state 管理 active project/selection；目前不引入 Zustand 或 TanStack Query。列表只 virtualize DOM，不引入 server paging。

## Website publication architecture

Rust `publish` 是 Cloudflare 發佈的單一 deep module。公開 Tauri seam 只有 state、connect／disconnect、settings、preview、publish、cancel 與 cleanup retry；React 經 `src/lib/tauri.ts` 的 Zod DTO 呼叫，不持有 Token、不讀 project media，也不直接發 HTTP。`PublishRuntime` 將 Token 保存至平台 credential store，失敗才使用 process-memory fallback，並集中管理 active account 與 cancellation flag。

`ProjectSession` 在 project lock 內建立完整 `PublishSnapshot`，包含 project ordering 的 live aggregates、sense-image metadata 與 sense／example audio metadata；Cloudflare I/O 在 background executor 執行，不長時間持有 session mutex。Snapshot builder 依 publisher 選擇過濾 forms、notes 與 relations，Primary form 同時負責唯一 Unicode slug 與 relation headword。POS 只輸出於 sense、圖片只輸出於 sense，音檔只輸出於 sense 或 example。只有 info Markdown 轉 HTML；其他 notes 全部作純文字 JSON。

正式 public template 位於 `src-tauri/templates/publish-site/` 並由 Rust `include_str!` 納入 binary；被 `.gitignore` 排除的 prototype 不是 runtime dependency。`schemaVersion: 1` corpus 與 template 一起產生，Static Assets manifest 使用 Cloudflare 規定的 base64-content-plus-extension SHA-256 前 32 hex。Direct Upload 以 upload JWT 上傳 Cloudflare 指定 buckets，完成後以 completion JWT 原子部署同一 Worker；Worker 綁定 `ASSETS` 與 `MEDIA` R2 bucket。

R2 bucket 固定由 Worker 名稱衍生為 `<worker-name>-media`；Rust 在載入與儲存設定時重算，frontend 只能顯示衍生值。R2 物件固定為 `media/images/<sha256>.png` 與 `media/audio/<sha256>.webm`。本機讀取前 canonicalize project path 並重算 SHA-256；遠端 bucket 必須具有相同 project ID 的 `.bkuw/owner.json`，Worker settings 的 `workers/tag` 也必須相同，否則 fail closed。媒體 Worker 只處理 `/media/`，支援 GET、HEAD、Range、ETag、正確 Content-Type 與 immutable cache。Cloudflare client 對 429／5xx 及暫時性網路錯誤做有界 retry 並尊重秒數形式 `Retry-After`；Worker module multipart 一律帶入明確 filename，未知 deployment 結果先讀 ownership state，再以精確 corpus SHA-256 health check 決定是否進入 cleanup。

發佈成功前不刪遠端媒體。Health check 最多 60 秒，比對首頁、完整 corpus bytes digest 與至少一個媒體 HEAD；通過後才將已驗證 project bucket 的 stale `media/` keys 刪除。失敗的 delete 記錄 `cleanup_pending`，不撤回已成功部署的網站。Static Assets 單檔超過 25 MiB 在 preview 阻擋；v1 不分片。

## Verification strategy

- Rust integration tests 透過 project/database module interface 使用 temporary project 與真實 SQLite。
- Vitest + React Testing Library 測互動、autosave、translations、validation 與 nested editors。
- WebdriverIO Tauri service 執行主要 desktop workflow smoke test。
- GitHub Actions 先在 Ubuntu 執行 TypeScript、Rust format 與 frontend unit checks，再於 Windows x64 與 macOS Apple Silicon jobs 執行 platform-specific Clippy、Rust tests 與 release-mode desktop E2E；一般 CI 不建立或上傳 installer artifacts，也不建立 macOS Intel 產物。一般 Markdown 變更略過 application jobs；`src-tauri` 內的文件仍觸發 application 與 portable XeLaTeX test，後者也會在 CI workflow 變更時執行，同一 pull request 的舊 CI run 會取消。成功的 trusted `main` push CI 會觸發 release planner；planner 僅在 exact HEAD 相對第一個 parent 同步增加四個 canonical versions，且對應 tag 尚不存在時，才於 Windows／macOS jobs 建置並暫存 NSIS／DMG。受限 `contents: write` 的 final job 驗證檔名與 SHA-256，最後建立以 exact SHA 為 target、含自動 changelog 的 Draft Release；GitHub 只在人工 Publish 時 materialize tag。`publish-draft` module 可安全更新尚未 materialize tag 的草稿 target，或更新 tag 已指向相同 commit 的既有草稿；manual recovery 可重用指定 release run 的 installer artifacts，無須重新打包。發布前保留人工確認閘門。

### Export wizard and local TeX setup

`ExportDialog` keeps output intent, settings, preview and current step in component state; Overleaf maps to the existing `latex` export kind, without a database migration. Autosave and settings are flushed before preview, and returning to edit invalidates the preview. `LatexRequirements` only mounts for local PDF at the requirements step, ignores stale async responses, and rechecks on focus after installer handoff. The typed adapter validates all environment/progress DTOs with Zod. Dependency errors have translation keys in both locales.

`export::environment` owns executable/version checks, template-derived package checks through the engine's sibling `kpsewhich`, an isolated 30-second compilation probe, diagnostics, installer download/handoff, and script generation. Version checks time out after 10 seconds; package probes after 5 seconds. The probe uses managed font files and no lexical data. Export snapshots are taken under the project lock; probing, network, and compilation run outside it. Existing `detect_xelatex` remains compatible with callers; normal compilation keeps its existing isolation and snapshot validation.

Installer download is explicitly requested, restricted to catalog HTTPS URLs with redirects disabled, streamed to an app-private temporary file, and verified against a pinned SHA-256 before launch. Progress is throttled to 200 ms. A process-wide AppState guard prevents overlapping downloads; cancellation is observed between chunks, with network reads bounded to 30 seconds. Partial downloads are deleted automatically. Verified installers live in unique `latex-installers` subdirectories and may be deleted after installation. macOS uses `/usr/bin/open` for the verified `.pkg`; Windows launches the verified `.exe` with the matching historical repository. No frontend shell/HTTP/filesystem capability is introduced. Installer handoff is not installation success; existing TeX blocks a new install. Script exports use the same catalog and user-selected output directory.

Catalog provenance: MacTeX `mactex-20260324.pkg` SHA-256 comes from Homebrew's `Casks/m/mactex.rb`; the TeX Live 2025 final installer was hashed from the official historic mirror and cross-checked against its published SHA-512. Catalog changes require revalidation; an unavailable or replaced download fails closed rather than falling back to an unverified installer. The installer itself manages package retrieval after handoff. Official package-manager links guide repair of an existing environment.

The real XeLaTeX smoke test can reuse an existing verified font cache through `BKUW_LATEX_SMOKE_FONT_CACHE`; when unset, it installs fonts in its temporary test directory. The fixture includes Traditional Chinese, IPA and a sense photo. Desktop E2E reuses packs only after `list_font_packs` reports successful integrity verification.
