# bkuw

`bkuw` 是供語言田野工作使用的 local-first 詞彙資料庫，支援 Windows x64 與 macOS Apple Silicon。專案資料保存在本機 SQLite，不需登入；建立、編輯、搜尋與匯出都可離線完成。使用者也可以主動把辭典發佈到自己的 Cloudflare 帳號。

## 功能

- 自訂書寫系統、多義項、義項詞性、多表記例句與 root/base 關係。
- Unicode／IPA 搜尋、自動儲存、刪除還原，以及英文／台灣繁中介面。
- 義項相片；義項與例句可匯入多個音檔或直接錄音。
- 從 UTF-8 CSV 建立專案，並匯出 rngagi-corpus CSV、XeLaTeX 原始碼、Overleaf ZIP 或 PDF。
- 自訂字母表、語意類別分組與手動排序；工作區、匯出辭典與公開網站使用同一排序。
- 透過引導精靈發佈或更新 Cloudflare `workers.dev` 辭典網站。

## 開始使用

1. 從 [GitHub Releases](https://github.com/rngagi/bkuw/releases) 下載 Windows x64 安裝程式或 macOS Apple Silicon DMG。
2. 開啟 bkuw，建立新專案、從 CSV 建立專案，或開啟既有 `.bkuw` 目錄。
3. 設定主要書寫系統後開始新增詞條。有效變更會自動儲存，也可按 `Ctrl/Cmd+S` 立即儲存。

操作、附件、排序、備份與網站發佈流程見[使用指南](docs/user-guide.md)。PDF、Overleaf 與字型準備見[匯出指南](docs/export-guide.md)。

## 專案與備份

每個 `.bkuw` 專案都是普通目錄，包含 SQLite 資料庫及 project-local 媒體。備份前先關閉專案，再完整複製整個目錄。CSV 是交換格式，不是專案備份。

```text
MyLanguage.bkuw/
├── .bkuw.lock
├── project.sqlite
├── media/
│   ├── images/
│   └── audio/
└── backups/
```

## 開發

需要 Node.js 24 LTS、pnpm 11、Rust stable 及對應平台的 Tauri 建置工具。

```bash
pnpm install
pnpm audio:prepare
pnpm tauri dev
```

完整環境、驗證命令與目錄說明見[開發指南](docs/development.md)。版本與安裝包只依[發布流程](docs/distribution.md)準備。

## 文件

[文件索引](docs/README.md)說明每份文件的職責；版本差異記錄在[變更紀錄](CHANGELOG.md)。公開版本、日期與安裝包以 [GitHub Releases](https://github.com/rngagi/bkuw/releases) 為準。
