# bkuw

`bkuw` 是一套為語言田野工作設計的 local-first 詞彙資料庫。它以 lexical entry 為核心，支援動態 writing systems、多義項、多表記例句、root/base 關係、Unicode/IPA 搜尋，以及 corpus CSV、XeLaTeX、Overleaf ZIP 與 PDF 匯出。

目前版本為 `0.2.3`。v0.2 export milestone 已完成本機 Rust／frontend 與真實 XeLaTeX 驗證；v0.2.1 修正 autosave focus，v0.2.2 顯示 XeLaTeX 診斷路徑並修正 Windows 關閉視窗權限，v0.2.3 改由 bkuw 下載、驗證、快取並隨匯出附上可攜字型。GitHub Actions 以 Windows x64 與 macOS Apple Silicon 建置安裝包；version tag 通過全部 CI 後會建立附 checksums 與雙語說明的 Draft Release。介面支援英文與台灣繁中，macOS Intel 不在支援與建置範圍內。

## 技術組成

- Tauri 2、Rust、SQLite
- React、TypeScript、Vite
- Tailwind CSS、Radix UI、Lucide
- React Hook Form、Zod、react-i18next

## 開發環境

- Node.js 24 LTS
- pnpm 11（實際版本由 `packageManager` 鎖定）
- Rust stable
- macOS：Xcode Command Line Tools
- Windows：MSVC Build Tools 與 WebView2

## 常用命令

```bash
pnpm install
pnpm tauri dev
pnpm check
pnpm test
pnpm test:rust
pnpm test:e2e:build
pnpm test:e2e
pnpm tauri build --no-bundle
pnpm tauri build
```

## 文件

- [產品規格](docs/product-spec.md)
- [架構與資料模型](docs/architecture.md)
- [v0.2 執行清單](plan.md)
- [rngagi-corpus CSV 契約](docs/corpus-csv-contract.md)
- [匯出指南／Export guide](docs/export-guide.md)
- [CI 安裝包與簽章](docs/distribution.md)
- [Roadmap](docs/roadmap.md)
- [Agent 工作規範](AGENTS.md)

## 專案資料

每個 project 是可手動備份的普通目錄：

```text
MyLanguage.bkuw/
├── .bkuw.lock
├── project.sqlite
└── backups/
```

`.bkuw.lock` 只用來防止兩個程序同時寫入同一專案。所有使用者資料都保存在本機，不需要帳號、伺服器或網路連線。

## v0.2 匯出

App header 的「匯出／Export」會先 flush autosave，再依序保存 profile、顯示 validation preview、選擇目的地並產生輸出。CSV 固定遵循 rngagi-corpus v0.3 九欄契約；LaTeX 會產生可編輯資料夾與 Overleaf-ready ZIP。本機有 XeLaTeX 時可一併建立 PDF，否則不影響來源與 ZIP 匯出。

LaTeX/PDF 不再依賴作業系統已安裝的字型。bkuw 會從官方固定版本下載 TeX Gyre Termes、Charis SIL、Noto Serif 與 Noto Serif CJK TC，驗證 SHA-256 後存入 app-private cache，並把實際使用的字型與授權檔放入來源資料夾和 ZIP。TeX Gyre Termes 是所有 LaTeX/PDF 匯出的必要 pack；缺少或驗證失敗時會阻擋匯出。Phonemic／phonetic（IPA）書寫系統固定使用 Charis SIL；現階段不提供 Thai／Tibetan 專用 managed font packs。

目前只修改 `bkuw` repository，尚未建立與 `rngagi-corpus` 的跨 repository 自動 contract test。若 corpus template 或版本改變，必須依 [CSV 契約](docs/corpus-csv-contract.md)人工重驗並更新 golden fixture。
