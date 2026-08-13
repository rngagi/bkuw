# bkuw 執行清單

本文件只保留目前基線、尚未發布的工作與候選 backlog。已發布版本的詳細變更由 Git history、GitHub Releases 與 `docs/roadmap.md` 保存，不在這裡重複整份歷史驗收紀錄。

規則：只有完成條件成立，且列出的驗證通過後才能勾選。行為或架構改變時，同步更新 `docs/product-spec.md` 與 `docs/architecture.md`。

## 目前基線

- [x] Local-first project lifecycle、SQLite migrations／backup／lock 與 typed Tauri adapter。
- [x] Dynamic writing systems、multi-form entries、sense-level POS、examples 與 root/base relations。
- [x] Unicode-safe autosave、IME composition handling、soft delete／Undo 與英文／台灣繁中 UI。
- [x] Project alphabet、entry section override、opt-in manual ordering 與 virtualized entry list。
- [x] rngagi-corpus v0.3 CSV、portable XeLaTeX／Overleaf ZIP／PDF 與 managed font packs。
- [x] Form／sense 搜尋、精簡詞表摘要、sense-level 相片與 optional LaTeX／PDF photo export。
- [x] Windows x64／macOS Apple Silicon CI、NSIS／DMG artifacts 與 tag-gated Draft Release。

最新公開版本為 `v0.4.0`。macOS Intel 不在支援或建置範圍內。

## 候選 backlog

以下尚未承諾版本或優先順序；開始實作前須先確認 scope 與 acceptance criteria。

- [ ] CSV import 與 versioned bkuw → rngagi-corpus upload workflow。
- [ ] bkuw／rngagi-corpus cross-repository contract fixture 與 CI。
- [ ] Audio import、playback 與 optional recording。
- [ ] 多 analysis-language gloss／translation。
- [ ] Example search、進階 FTS 與 fuzzy search。
- [ ] IPA helper、tags、filters 與 duplicate detection。
- [ ] Trash／backup manager。
- [ ] Production signing、Apple notarization 與 auto-update。

Cloud sync、accounts、authentication、collaboration、server backend、mobile、ASR、ELAN-style timeline、waveform segmentation、Git syncing 與自動上傳 lexical data 仍屬明確排除範圍，除非另行核准產品方向。

## 驗證命令

一般行為變更：

```bash
pnpm check
pnpm test
pnpm test:rust
```

Milestone／release 前另執行：

```bash
pnpm test:e2e:build
pnpm test:e2e
pnpm tauri build --no-bundle
```

Release tag 必須與 package、Cargo、Cargo lock、Tauri app version 一致；遠端僅驗收 Windows x64 與 macOS Apple Silicon。
