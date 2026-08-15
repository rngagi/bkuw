# bkuw

`bkuw` 是一套為語言田野工作設計的 local-first 詞彙資料庫。它以 lexical entry 為核心，支援動態 writing systems、多義項、多表記例句、義項相片、root/base 關係、Unicode/IPA 搜尋，以及 corpus CSV、XeLaTeX、Overleaf ZIP 與 PDF 匯出。

目前版本為 `0.4.2`。詞表會把 IPA 放在主要表記同一行，保留各義項自己的詞性與簡釋，最多顯示兩列摘要；搜尋同時涵蓋表記、簡釋與定義。義項相片可在 app 內預覽，LaTeX／PDF 匯出會使用適合雙欄印刷的衍生圖以減少檔案大小。Windows `Ctrl`／macOS `Cmd` 搭配 `-`、`=`、`0` 可調整或重設介面縮放。

Project 可定義含 multigraph 的字母表，並提供自動／詞條小標覆寫／完整手動拖拉三種排序層級；工作區與匯出辭典共用同一排序結果。

XeLaTeX template 使用較寬鬆行距、欄內小標、橢圓例句標記與一層 direct root/base related entries；相關詞可在 export profile 關閉或選擇 root、base、兩者。

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

## 介面縮放

Windows 可使用 `Ctrl+-`／`Ctrl+=` 縮小或放大，`Ctrl+0` 回到 100%；macOS 使用對應的 `Cmd` 快捷鍵。bkuw 會在這台裝置保存縮放比例，方便配合 Windows 顯示縮放與不同 DPI 的螢幕。

## 文件

- [產品規格](docs/product-spec.md)
- [架構與資料模型](docs/architecture.md)
- [目前執行清單與 backlog](plan.md)
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
├── media/
│   └── images/
└── backups/
```

`.bkuw.lock` 只用來防止兩個程序同時寫入同一專案。所有使用者資料都保存在本機，不需要帳號、伺服器或網路連線。

義項可加入 PNG、JPEG 或 WebP 相片。bkuw 在本機以 Canvas 解碼，長邊超過 2560px 時等比例縮小，再統一保存為 `media/images/*.png`；資料庫只保存相對路徑、尺寸、原始檔名與 SHA-256。此為保留細節的輕度處理，不設定 500 KiB 硬性上限。

## 匯出

App header 的「匯出／Export」會先 flush autosave，再依序保存 profile、顯示 validation preview、選擇目的地並產生輸出。CSV 固定遵循 rngagi-corpus v0.3 九欄契約；LaTeX 可選擇是否包含義項相片，並產生可編輯資料夾與 Overleaf-ready ZIP。本機有 XeLaTeX 時可一併建立 PDF，否則不影響來源與 ZIP 匯出。

LaTeX/PDF 不再依賴作業系統已安裝的字型。bkuw 會從官方固定版本下載 TeX Gyre Termes、Charis SIL、Noto Serif、Noto Serif CJK TC、[Chiron Sung HK](https://github.com/chiron-fonts/chiron-sung-hk) 與 [Chiron Hei HK](https://github.com/chiron-fonts/chiron-hei-hk)，驗證 SHA-256 後存入 app-private cache，並把實際使用的字型與授權檔放入來源資料夾和 ZIP。Chiron Sung HK 是明體／宋體風格，Chiron Hei HK 是黑體／無襯線風格。TeX Gyre Termes 是所有 LaTeX/PDF 匯出的必要 pack；缺少或驗證失敗時會阻擋匯出。Phonemic／phonetic（IPA）書寫系統固定使用 Charis SIL；現階段不提供 Thai／Tibetan 專用 managed font packs。

目前只修改 `bkuw` repository，尚未建立與 `rngagi-corpus` 的跨 repository 自動 contract test。若 corpus template 或版本改變，必須依 [CSV 契約](docs/corpus-csv-contract.md)人工重驗並更新 golden fixture。
