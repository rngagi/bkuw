# bkuw

`bkuw` 是為語言田野工作設計的 local-first 詞彙資料庫，支援 Windows x64 與 macOS Apple Silicon。所有專案資料保存在本機，不需帳號或伺服器，日常編輯可離線使用。公開版本與安裝包以 [GitHub Releases](https://github.com/rngagi/bkuw/releases) 為準。

## 主要功能

- 自訂書寫系統、多義項、義項詞性、多表記例句與 root/base 關係。
- Unicode／IPA、表記與義項搜尋、自動儲存、刪除還原，以及英文／台灣繁中介面。
- 義項相片；義項與例句可匯入多個音檔或直接錄音，提供試聽、重錄、播放與進度拖曳。
- 從 UTF-8 CSV 建立專案，支援欄位對應、相鄰列分組與匯入預覽。
- 自訂字母表、語意類別分組與手動排序；工作區和匯出辭典共用排序結果。
- rngagi-corpus CSV、XeLaTeX 原始碼、Overleaf ZIP 與本機 PDF 匯出。

啟動後可建立專案、從 CSV 建立專案或開啟既有專案。字型狀態在背景檢查，語言選單旁提供管理入口；缺少匯出字型不會阻擋專案編輯。匯出所需字型可另外下載，完整流程見 [匯出指南](docs/export-guide.md)。

Windows 使用 `Ctrl+-`／`Ctrl+=` 調整介面縮放、`Ctrl+0` 重設；macOS 使用對應的 `Cmd` 快捷鍵，比例會保存在這台裝置。

## 專案資料

每個 project 是普通目錄；備份時請先關閉專案，再完整複製專案目錄，CSV 無法取代專案備份。

```text
MyLanguage.bkuw/
├── .bkuw.lock
├── project.sqlite
├── media/
│   ├── images/    # 義項相片，PNG
│   └── audio/     # 義項與例句音檔，WebM／Opus
└── backups/
```

`.bkuw.lock` 防止兩個程序同時寫入。相片接受 PNG／JPEG／WebP，過大圖片在本機輕度縮圖後保存為 PNG；音檔接受 WAV、MP3、M4A/AAC、FLAC、OGG/Opus、AIFF、WebM，匯入與錄音統一由本機工具轉為 WebM／Opus。來源檔不被修改，附件不依賴外部路徑。格式、大小與錄音限制見 [產品規格](docs/product-spec.md)。

## 開發環境

技術組成：Tauri 2、Rust、SQLite、React、TypeScript、Vite、Tailwind CSS、Radix UI、Lucide、React Hook Form、Zod 與 react-i18next。

- Node.js 24 LTS、pnpm 11，確切版本由 `packageManager` 鎖定。
- Rust stable。
- macOS Apple Silicon：Xcode Command Line Tools、make。
- Windows x64：MSVC Build Tools、WebView2、MSYS2 MINGW64 的 gcc／make／pkgconf／curl／tar／xz。

```bash
pnpm install
pnpm audio:prepare
pnpm tauri dev
```

`audio:prepare` 首次下載經 SHA-256 驗證的 FFmpeg／Opus 原始碼並建置工具，之後可離線使用。`tauri dev` 與一般 build 會自動檢查；直接執行 Rust 音檔測試前也需準備工具。Windows 預設使用 `C:/msys64`，自訂安裝位置可設定 `BKUW_MSYS2_LOCATION`；CI 使用 MSYS2 setup 回傳的實際位置。音訊工具建置不需要 Python。安裝包隨附工具、授權與完整來源，使用者不需另行安裝。

驗證與 milestone 命令見 [執行清單](plan.md#驗證命令)；打包與版本準備見 [發布流程](docs/distribution.md)。

## 文件導覽

| 文件 | 維護內容 |
|---|---|
| [產品規格](docs/product-spec.md) | 現行功能、限制與操作行為 |
| [架構與資料模型](docs/architecture.md) | 模組界線、資料流、交易與驗證策略 |
| [執行清單](plan.md) | 最近工作、未完成驗收與唯一的候選 backlog |
| [變更紀錄](CHANGELOG.md) | 最新提交摘要與歷史版本變更；發布狀態以 GitHub Releases 為準 |
| [CSV 契約](docs/corpus-csv-contract.md) | rngagi-corpus v0.3 九欄對應、資訊損失與人工重驗要求 |
| [匯出指南](docs/export-guide.md) | PDF、Overleaf、字型與 LaTeX 安裝的雙語操作說明 |
| [發布流程](docs/distribution.md) | CI、安裝包、Draft Release、復原與簽章 |
| [Agent 工作規範](AGENTS.md) | 開發與維護必須遵守的規則 |

匯出包內的 [README](src-tauri/templates/latex/README.md) 與 [INSTALL](src-tauri/templates/latex/INSTALL.md) 會隨 LaTeX 來源交付，獨立保留；[音訊測試素材說明](src-tauri/tests/fixtures/audio/README.md) 記錄 fixture 來源。
