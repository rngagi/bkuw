# rngagi-corpus v0.3 CSV 契約

本文件固定 bkuw v0.2 的外部 CSV contract。輸出為 UTF-8、無 BOM、RFC 4180 quoting 與 CRLF record terminator，欄位順序不可變：

```text
form,gloss_zh,word_root,example,example_translation_zh,ipa,part_of_speech,gloss_en,notes
```

## 映射

| 欄位 | bkuw 資料 |
|---|---|
| `form` | entry primary writing-system form |
| `gloss_zh` | sense gloss；不得以 definition 代替 |
| `word_root` | root target primary form或 fallback；多值以 `;` 連接 |
| `example` | profile 指定 writing system 中第一個同時具有文字與翻譯的 example |
| `example_translation_zh` | 被選 example 的單一 translation |
| `ipa` | profile 指定 phonetic／phonemic form；不包含顯示用 `[]`／`//` |
| `part_of_speech` | project POS mapping 對應的七種 corpus vocabulary |
| `gloss_en` | v0.2 固定留空 |
| `notes` | stable labels 合併 entry notes、sense definition、semantic domain、example notes |

所有未 soft-delete entries 都會處理，每個 sense 一列。排序先使用 profile language tag 的 ICU4X collation 比較 primary form，再以 entry UUID 與 sense order 穩定決定次序。

## 阻擋與警告

下列情況阻擋輸出：analysis language 不是 `zh-TW`、entry 無 sense、缺 primary form、sense 缺 gloss、root form 包含 `;`、或最終沒有 rows。

下列情況保留輸出但顯示 warning：未採用的 examples／example forms、base relations、example 缺文字或翻譯、POS 未 mapping、重複 `form + gloss_zh` candidate。

此九欄格式無法無損表示 entry UUID、base relations、多 examples、多 example writing systems、`gloss_en` 或多 analysis-language translations。不要把匯出的 CSV 當作 bkuw project backup；SQLite project 才是唯一真相。

## 相容性狀態

目前只修改 `bkuw` repository，沒有 `rngagi-corpus` cross-repository automated contract test，也不會由 CI 上傳資料或修改 corpus repository。若 rngagi-corpus template／version 改變，維護者必須人工重新核對九欄契約、更新 Rust golden fixture，再執行完整 CSV tests。
