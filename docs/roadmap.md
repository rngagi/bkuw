# bkuw roadmap

## 已完成

### v0.1 / Core editor

Local-first project lifecycle、dynamic writing systems、lexical aggregates、multi-writing-system examples、Unicode search、autosave、soft delete／Undo、英文與台灣繁中 UI。

### v0.2 / Export

rngagi-corpus v0.3 九欄 CSV、versioned export profile、可編輯 XeLaTeX project、Overleaf-ready ZIP、optional local PDF、ICU4X sorting、reverse index 與 portable font presets。

目前 CSV 相容性由 bkuw 內的 golden fixture 與人工 contract review 保護；尚無跨 `bkuw`／`rngagi-corpus` repositories 的自動 contract test。

v0.2.2 加入 tag-gated GitHub Release pipeline：全部 CI jobs 通過後自動建立含 Windows x64 NSIS、macOS Apple Silicon DMG、checksums 與 generated notes 的 unsigned Draft Release，再由 maintainer 發布。

v0.2.3 加入 bkuw-managed portable font packs：固定官方來源與 SHA-256、app-private cache、匯出內含 fonts/licenses、TeX Gyre Termes fatal requirement，並將 IPA 固定為 Charis SIL。

### v0.3 / Dictionary ordering and LaTeX refresh

Project-defined alphabet、entry section override 與 opt-in manual drag ordering；workspace 與匯出辭典共用順序及小標。XeLaTeX template 改善行距、詞條註解、IPA 詞頭、例句與 optional direct root/base related entries。Export snapshot 使用 bulk loading，長時間工作移至 background executor 並顯示階段進度。

v0.3.1 新增 Chiron Sung HK／Chiron Hei HK managed portable fonts 與明體／黑體風格提示；詞表將 IPA 合併至主要表記同行，並依序呈現每個義項自己的詞性與簡釋。

### v0.4 / Sense photos and search refinement

工作區搜尋擴充至 sense gloss 與 definition，並以 migration 5 回填 Unicode-safe search keys。詞表摘要最多顯示兩列，更多義項改顯示總數；LaTeX 辭典標題使用深紅粗體。

Sense-level 相片接受 PNG／JPEG／WebP，在本機 Canvas 輕度縮圖後統一保存為 project-relative PNG。Migration 6 保存圖片 metadata 與 SHA-256；LaTeX／PDF profile 可選擇是否把相片加入來源資料夾、Overleaf ZIP 與成品。

v0.4.1 修正 React field-array UI key 覆蓋持久化 sense ID，導致相片上傳誤報找不到義項的問題；同時細分詞條、義項與相片不存在的雙語錯誤訊息。CI／release 流程改為 version tag 重用完全相同 commit SHA 的成功 `main` artifacts，不再重複測試與平台打包。

v0.4.3 修正 WebView CSP 阻擋 sense 相片預覽的問題，並在預覽失敗時顯示雙語錯誤；LaTeX／PDF 匯出將相片縮入 `1000×900px`，不透明圖使用品質 82 JPEG、透明圖保留 PNG，減少雙欄辭典 PDF 體積且不改寫 project-local PNG。另新增 Windows `Ctrl+-/=/0`、macOS `Cmd+-/=/0` 的持久化 app zoom。發布流程新增單一 version preparation command；一般 push CI 不產生安裝包，只有 version commit 的 exact-SHA CI 成功後才自動建置 NSIS／DMG，在兩個平台完成後建立 checksums 與 exact-SHA Draft Release，人工 Publish 時才 materialize tag，並可從失敗 run 重用既有 installer artifacts。

## 候選 backlog

- Audio import/playback 與 optional recording。
- CSV import 與經雙方版本化的 bkuw → rngagi-corpus upload workflow。
- Cross-repository contract fixture／CI；需兩個 repositories 共同確認後才建立。
- 多 analysis-language translations、進階 FTS、example search、language-specific collation controls。
- IPA helper、tags、filters、duplicate detection、backup manager。
- Production signing、notarization 與 auto-update。
