# bkuw

`bkuw` 是一套為語言田野工作設計的 local-first 詞彙資料庫。它以 lexical entry 為核心，支援動態 writing systems、多義項、多表記例句、root/base 關係與 Unicode/IPA 搜尋。

Milestone 1 的功能與本機驗收已完成；Windows x64 與 macOS arm64/x64 的 GitHub Actions 驗證會在 push 後執行。介面支援英文與台灣繁中。

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
- [Milestone 1 執行清單](plan.md)
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
