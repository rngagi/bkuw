# bkuw roadmap

## v0.1 / Milestone 1

Local-first project lifecycle、dynamic writing systems、lexical aggregates、multi-writing-system examples、Unicode search、autosave、soft delete／Undo、英文與台灣繁中 UI。

## v0.2 / Export milestone

rngagi-corpus v0.3 九欄 CSV、versioned export profile、可編輯 XeLaTeX project、Overleaf-ready ZIP、optional local PDF、ICU4X sorting、reverse index 與 portable font presets。

目前 CSV 相容性由 bkuw 內的 golden fixture 與人工 contract review 保護；尚無跨 `bkuw`／`rngagi-corpus` repositories 的自動 contract test。

v0.2.2 加入 tag-gated GitHub Release pipeline：全部 CI jobs 通過後自動建立含 Windows x64 NSIS、macOS Apple Silicon DMG、checksums 與 generated notes 的 unsigned Draft Release，再由 maintainer 發布。

v0.2.3 加入 bkuw-managed portable font packs：固定官方來源與 SHA-256、app-private cache、匯出內含 fonts/licenses、TeX Gyre Termes fatal requirement，並將 IPA 固定為 Charis SIL。

## v0.3 / Dictionary ordering and LaTeX refresh

Project-defined alphabet、entry section override 與 opt-in manual drag ordering；workspace 與匯出辭典共用順序及小標。XeLaTeX template 改善行距、詞條註解、IPA 詞頭、例句與 optional direct root/base related entries。Export snapshot 使用 bulk loading，長時間工作移至 background executor 並顯示階段進度。

v0.3.1 新增 Chiron Sung HK／Chiron Hei HK managed portable fonts 與明體／黑體風格提示；詞表將 IPA 合併至主要表記同行，並依序呈現每個義項自己的詞性與簡釋。

## 後續候選

- Audio import/playback 與 optional recording。
- CSV import 與經雙方版本化的 bkuw → rngagi-corpus upload workflow。
- Cross-repository contract fixture／CI；需兩個 repositories 共同確認後才建立。
- 多 analysis-language translations、進階 FTS、example search、language-specific collation controls。
- IPA helper、tags、filters、duplicate detection、backup manager。
- Production signing、notarization 與 auto-update。
