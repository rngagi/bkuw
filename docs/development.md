# bkuw 開發指南

## 環境

- Node.js 24 LTS；pnpm 版本由 [`package.json`](../package.json) 的 `packageManager` 鎖定。
- Rust stable。
- macOS Apple Silicon：Xcode Command Line Tools、make。
- Windows x64：MSVC Build Tools、WebView2，以及 MSYS2 MINGW64 的 gcc、make、pkgconf、curl、tar、xz。

```bash
pnpm install
pnpm audio:prepare
pnpm tauri dev
```

`pnpm audio:prepare` 依 [`scripts/audio/prepare.sh`](../scripts/audio/prepare.sh) 下載、驗證並建置固定版本的 FFmpeg／Opus。Windows 預設從 `C:/msys64` 尋找 MSYS2；其他位置使用 `BKUW_MSYS2_LOCATION`。

## 驗證

```bash
pnpm check
pnpm test
pnpm test:rust
pnpm test:e2e:build
pnpm test:e2e
pnpm tauri build --no-bundle
```

前三項是每次行為變更的基本檢查；desktop milestone 另執行 E2E 與 no-bundle build。真實 XeLaTeX portable template smoke test 由 CI 執行，也可執行 [`src-tauri/src/database.rs`](../src-tauri/src/database.rs) 中 ignored 的 `portable_xelatex_template_compiles_with_the_real_engine` test。

## 程式碼入口

- [`src/lib/tauri.ts`](../src/lib/tauri.ts)：React 唯一的 typed Tauri adapter。
- [`src/types/domain.ts`](../src/types/domain.ts)：frontend DTO 與 Zod schema。
- [`src-tauri/src/database.rs`](../src-tauri/src/database.rs)：project lifecycle、SQLite 與 aggregate transaction。
- [`src-tauri/src/domain.rs`](../src-tauri/src/domain.rs)：共享 DTO；包含 `ExportSettingsV1`、`EntrySortSettingsV2` 與 `PublishSettingsV1`。
- [`src-tauri/src/export.rs`](../src-tauri/src/export.rs)：CSV、LaTeX、ZIP、PDF snapshot 與 renderer。
- [`src-tauri/src/publish.rs`](../src-tauri/src/publish.rs)：Cloudflare client、publication snapshot、R2、Static Assets 與網站產生。
- [`src-tauri/templates/`](../src-tauri/templates)：LaTeX 與公開網站的 tracked 產生範本。

架構規則與資料流見[架構文件](architecture.md)。

## 版本與發布

不要直接修改版本檔或建立 tag。版本準備只使用：

```bash
pnpm release:prepare -- <version>
```

後續提交、CI、Draft Release 與復原程序見[發布流程](distribution.md)。
