import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  en: {
    translation: {
      app: { name: "bkuw", tagline: "Arrange words. Keep the fieldwork local." },
      common: {
        add: "Add", cancel: "Cancel", close: "Close", create: "Create", delete: "Delete",
        open: "Open", save: "Save", settings: "Settings", remove: "Remove", undo: "Undo",
        moveUp: "Move up", moveDown: "Move down", none: "None", loading: "Loading…", details: "Details",
      },
      start: {
        title: "Your lexical projects, stored on this device",
        body: "Create a project for a language or open an existing .bkuw folder.",
        createProject: "Create project", openProject: "Open project",
        parentFolder: "Parent folder", chooseFolder: "Choose folder", projectName: "Project name",
        languageName: "Language name", languageCode: "ISO 639-3 language code",
        languageCodeHelp: "Optional three-letter code, for example yue or bod.",
        lookupLanguageCode: "Look up ISO 639-3 codes",
        projectExistsTitle: "Project name already exists",
        projectExistsBody: "A .bkuw project with this name already exists in the selected folder. Choose another name or open the existing project.",
      },
      workspace: {
        newEntry: "New entry", search: "Search every lexical form…", noEntries: "No entries yet",
        noMatch: "No matching entries", selectEntry: "Select an entry or create a new one.",
        closeProject: "Close project", untitled: "Untitled entry", saved: "Saved", saving: "Saving…",
        unsaved: "Unsaved changes", saveFailed: "Save failed", deleted: "Entry deleted",
      },
      entry: {
        forms: "Forms", addForm: "Add form", writingSystem: "Writing system", text: "Text",
        notes: "Entry notes", senses: "Senses", addSense: "Add sense", sense: "Sense {{number}}",
        gloss: "Gloss", definition: "Definition", partOfSpeech: "Part of speech",
        semanticDomain: "Semantic domain", examples: "Examples", addExample: "Add example",
        example: "Example {{number}}", translation: "Translation", exampleNotes: "Example notes",
        addExampleForm: "Add example form", relations: "Root and base", addRelation: "Add relation",
        relationType: "Relation type", root: "Root", base: "Base", linkedEntry: "Linked entry",
        fallbackText: "Unlinked form", fallbackHelp: "Use this when the root or base is not yet an entry. bkuw also keeps it as a readable label if a linked entry is later removed.",
        navigate: "Open linked entry", deleteEntry: "Delete entry", deleteTitle: "Delete this entry?",
        deleteBody: "The entry will disappear from the list. You can undo immediately after deletion.",
        confirmDelete: "Delete entry", saveErrorTitle: "Could not save this entry",
      },
      settings: {
        title: "Project settings", description: "Description", writingSystems: "Writing systems",
        addWritingSystem: "Add writing system", name: "Name", type: "Type", role: "Display role",
        primary: "Primary", secondary: "Secondary", scriptCode: "Script code",
        languageTag: "Language tag", fontFamily: "Font family",
        type_orthography: "Orthography", type_romanization: "Romanization", type_transliteration: "Transliteration",
        type_phonemic: "Phonemic", type_phonetic: "Phonetic", type_other: "Other",
        onboardingTitle: "Set up this project's writing systems",
        onboardingBody: "Add each way the language will be written, then choose one primary form. Entries and examples will use these rows automatically.",
        writingSystemsHelp: "Examples: Traditional Chinese, Pinyin, IPA; or Tibetan, Wylie, IPA.",
        basicFields: "Writing system", advancedFields: "Optional technical details",
        scriptCodeHelp: "ISO 15924 script code, such as Hant, Latn, or Tibt.",
        languageTagHelp: "Optional BCP 47 tag, such as zh-Hant. Leave blank if unsure.",
        fontFamilyHelp: "Optional installed font family used to display this writing system.",
        metadataTitle: "Sense metadata",
        metadataHelp: "Define reusable choices once; senses will show dropdowns instead of asking you to retype them.",
        partOfSpeechOptions: "Parts of speech", semanticDomainOptions: "Semantic domains",
        addOption: "Add option", optionPlaceholder: "Type a value and press Add",
      },
      locale: { label: "Interface language", en: "English", zhTW: "繁體中文（台灣）" },
      error: {
        generic: "Something went wrong. Your unsaved draft has been kept.", validation: "Check the highlighted project data.",
        invalid_project: "This is not a valid bkuw project.", project_locked: "This project is already open in another bkuw process.",
        revision_conflict: "This entry changed after it was loaded. Reload before saving.",
        filesystem: "The project folder could not be accessed.", database: "The project database operation failed.",
        project_exists: "A project with this name already exists.",
        relation_target_required: "A root/base relation needs a linked entry or an unlinked form.",
        self_relation: "An entry cannot link to itself as a root or base.",
        unsupported_schema: "This project was created by a newer version of bkuw.",
        project_open: "Close the current project before opening another one.", no_project: "No project is open.",
        not_found: "The requested entry was not found.", internal: "bkuw could not access the active project session.",
      },
    },
  },
  "zh-TW": {
    translation: {
      app: { name: "bkuw", tagline: "整理詞語，讓田野資料留在本機。" },
      common: {
        add: "新增", cancel: "取消", close: "關閉", create: "建立", delete: "刪除", open: "開啟",
        save: "儲存", settings: "設定", remove: "移除", undo: "復原", moveUp: "上移", moveDown: "下移", details: "詳細資訊",
        none: "無", loading: "載入中…",
      },
      start: {
        title: "儲存在這台裝置上的詞彙專案", body: "為一種語言建立專案，或開啟既有的 .bkuw 資料夾。",
        createProject: "建立專案", openProject: "開啟專案", parentFolder: "上層資料夾",
        chooseFolder: "選擇資料夾", projectName: "專案名稱", languageName: "語言名稱", languageCode: "ISO 639-3 語言代碼",
        languageCodeHelp: "選填的三字母代碼，例如 yue 或 bod。", lookupLanguageCode: "查詢 ISO 639-3 代碼",
        projectExistsTitle: "專案名稱已存在",
        projectExistsBody: "所選資料夾中已有同名的 .bkuw 專案。請改用其他名稱，或開啟既有專案。",
      },
      workspace: {
        newEntry: "新增詞條", search: "搜尋所有詞彙表記…", noEntries: "尚無詞條", noMatch: "找不到符合的詞條",
        selectEntry: "選取詞條或建立新詞條。", closeProject: "關閉專案", untitled: "未命名詞條", saved: "已儲存",
        saving: "儲存中…", unsaved: "尚未儲存", saveFailed: "儲存失敗", deleted: "已刪除詞條",
      },
      entry: {
        forms: "表記", addForm: "新增表記", writingSystem: "書寫系統", text: "文字", notes: "詞條註記",
        senses: "義項", addSense: "新增義項", sense: "義項 {{number}}", gloss: "簡釋", definition: "定義",
        partOfSpeech: "詞類", semanticDomain: "語義領域", examples: "例句", addExample: "新增例句",
        example: "例句 {{number}}", translation: "翻譯", exampleNotes: "例句註記", addExampleForm: "新增例句表記",
        relations: "詞根與詞基", addRelation: "新增關係", relationType: "關係類型", root: "詞根", base: "詞基",
        linkedEntry: "連結詞條", fallbackText: "未連結詞形",
        fallbackHelp: "詞根或詞基尚未建立成詞條時可填在這裡；已連結的詞條日後移除時，bkuw 也會保留這段可讀文字。",
        navigate: "開啟連結詞條", deleteEntry: "刪除詞條", deleteTitle: "要刪除這個詞條嗎？",
        deleteBody: "詞條會從清單移除；刪除後仍可立即復原。", confirmDelete: "刪除詞條",
        saveErrorTitle: "無法儲存這個詞條",
      },
      settings: {
        title: "專案設定", description: "說明", writingSystems: "書寫系統", addWritingSystem: "新增書寫系統",
        name: "名稱", type: "類型", role: "顯示角色", primary: "主要", secondary: "次要",
        scriptCode: "文字代碼", languageTag: "語言標籤", fontFamily: "字型",
        type_orthography: "正字法", type_romanization: "羅馬字", type_transliteration: "轉寫",
        type_phonemic: "音位", type_phonetic: "語音", type_other: "其他",
        onboardingTitle: "設定這個專案的書寫系統",
        onboardingBody: "加入這個語言會使用的每一種表記，並指定一個主要表記。詞條與例句之後會自動依這些設定建立欄位。",
        writingSystemsHelp: "例如：繁體中文、拼音、IPA；或藏文、威利轉寫、IPA。",
        basicFields: "書寫系統", advancedFields: "選填的技術資訊",
        scriptCodeHelp: "ISO 15924 文字代碼，例如 Hant、Latn 或 Tibt。",
        languageTagHelp: "選填的 BCP 47 標籤，例如 zh-Hant；不確定可留空。",
        fontFamilyHelp: "選填；使用裝置上已安裝的字型顯示這種表記。",
        metadataTitle: "義項 metadata", metadataHelp: "先集中定義可重用的選項；編輯義項時即可使用下拉選單，不必反覆輸入。",
        partOfSpeechOptions: "詞類", semanticDomainOptions: "語義領域",
        addOption: "新增選項", optionPlaceholder: "輸入內容後按新增",
      },
      locale: { label: "介面語言", en: "English", zhTW: "繁體中文（台灣）" },
      error: {
        generic: "發生錯誤，尚未儲存的草稿仍會保留。", validation: "請檢查專案資料。",
        invalid_project: "這不是有效的 bkuw 專案。", project_locked: "此專案已由另一個 bkuw 程序開啟。",
        revision_conflict: "載入後詞條已有變更，請重新載入再儲存。", filesystem: "無法存取專案資料夾。",
        database: "專案資料庫操作失敗。",
        project_exists: "已有同名專案。", relation_target_required: "詞根／詞基必須連結詞條或填入未連結詞形。",
        self_relation: "詞條不能將自己設為詞根或詞基。",
        unsupported_schema: "此專案由較新版本的 bkuw 建立。", project_open: "請先關閉目前的專案。",
        no_project: "目前沒有開啟的專案。", not_found: "找不到指定的詞條。", internal: "bkuw 無法存取目前的專案工作階段。",
      },
    },
  },
} as const;

const stored = window.localStorage.getItem("bkuw.locale");
const detected = window.navigator.language.toLowerCase().startsWith("zh") ? "zh-TW" : "en";

void i18n.use(initReactI18next).init({
  resources,
  lng: stored === "en" || stored === "zh-TW" ? stored : detected,
  fallbackLng: "en",
  interpolation: { escapeValue: false },
}).then(() => { document.documentElement.lang = i18n.resolvedLanguage ?? "en"; });

i18n.on("languageChanged", (locale) => {
  document.documentElement.lang = locale;
});

export async function setLocale(locale: "en" | "zh-TW") {
  window.localStorage.setItem("bkuw.locale", locale);
  await i18n.changeLanguage(locale);
}

export default i18n;
