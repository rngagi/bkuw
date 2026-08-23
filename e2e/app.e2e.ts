import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { $, browser, expect } from "@wdio/globals";

describe("bkuw desktop shell", () => {
  it("renders the detected locale and opens the project dialog", async () => {
    const heading = await $("h1");
    await expect(heading).toBeDisplayed();
    const initialHeading = await heading.getText();
    if (initialHeading.includes("可攜字型") || initialHeading.includes("portable fonts")) {
      const chineseFonts = initialHeading.includes("可攜字型");
      await $(chineseFonts ? "button=下載全部字型" : "button=Download all fonts").click();
      await browser.waitUntil(async () => !(await $("h1").getText()).includes(chineseFonts ? "可攜字型" : "portable fonts"), { timeout: 180_000, timeoutMsg: "portable font setup did not finish" });
    }
    const chinese = (await $("h1").getText()).includes("詞彙專案");
    await $(chinese ? "button=建立專案" : "button=Create project").click();
    await expect($("[role=dialog]")).toBeDisplayed();
    await expect($("[role=dialog] h2")).toHaveText(chinese ? "建立專案" : "Create project");
  });

  it("creates a new project through the typed CSV import commands", async () => {
    const parentDir = mkdtempSync(join(tmpdir(), "bkuw-csv-e2e-"));
    const sourcePath = join(parentDir, "source.csv");
    let projectOpen = false;
    try {
      writeFileSync(sourcePath, "form;gloss;example;translation;pos;domain;roots\né;first;sentence;translation;verb;Motion;r1|r2\né;second;;;;Motion;r1|r2\n", "utf8");
      const inspection = await browser.tauri.execute(
        ({ core }, path) => core.invoke("inspect_csv", { path, delimiter: null }),
        sourcePath,
      ) as any;
      expect(inspection.delimiter).toBe("semicolon");
      const primaryId = randomUUID();
      const previewRequest = {
        sourcePath,
        delimiter: inspection.delimiter,
        project: {
          parentDir,
          name: "CSV desktop smoke",
          languageName: "Test language",
          languageCode: null,
          analysisLanguage: "en",
          writingSystems: [{ id: primaryId, name: "Primary", type: "orthography", scriptCode: null, languageTag: "und", displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null }],
        },
        mappings: [
          { columnIndex: 0, target: { kind: "entryForm", writingSystemId: primaryId } },
          { columnIndex: 1, target: { kind: "senseGloss" } },
          { columnIndex: 2, target: { kind: "exampleForm", writingSystemId: primaryId } },
          { columnIndex: 3, target: { kind: "exampleTranslation" } },
          { columnIndex: 4, target: { kind: "partOfSpeech" } },
          { columnIndex: 5, target: { kind: "semanticDomain" } },
          { columnIndex: 6, target: { kind: "rootFallback" } },
        ],
        groups: [],
        excludedRows: [],
        rootDelimiter: "|",
      };
      const preview = await browser.tauri.execute(
        ({ core }, request) => core.invoke("preview_csv_import", { request }),
        previewRequest,
      ) as any;
      expect(preview.blockingErrorCount).toBe(0);
      expect(preview.importEntryCount).toBe(1);
      expect(preview.importSenseCount).toBe(2);
      const result = await browser.tauri.execute(
        ({ core }, request) => core.invoke("create_project_from_csv", { request }),
        { preview: previewRequest, previewToken: preview.previewToken },
      ) as any;
      projectOpen = true;
      expect(result.snapshot.entries).toHaveLength(1);
      expect(result.snapshot.partOfSpeechOptions).toEqual(["verb"]);
      expect(result.snapshot.semanticDomainOptions).toEqual(["Motion"]);
      const entry = await browser.tauri.execute(
        ({ core }, id) => core.invoke("load_entry", { id }),
        result.snapshot.entries[0].id,
      ) as any;
      expect(entry.forms[0].text).toBe("é");
      expect(entry.senses).toHaveLength(2);
      expect(entry.relations.map((relation: any) => relation.fallbackText)).toEqual(["r1", "r2"]);
    } finally {
      if (projectOpen) await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      rmSync(parentDir, { recursive: true, force: true });
    }
  });

  it("persists a Unicode aggregate across a real desktop project reopen", async () => {
    const parentDir = mkdtempSync(join(tmpdir(), "bkuw-e2e-"));
    let projectOpen = false;
    try {
      const snapshot = await browser.tauri.execute(
        ({ core }, request) => core.invoke("create_project", { request }),
        { parentDir, name: "Desktop smoke", languageName: "Traditional Chinese", languageCode: "zh-Hant" },
      ) as any;
      projectOpen = true;
      const primary = snapshot.writingSystems[0];
      const pinyin = { ...primary, id: randomUUID(), name: "Pinyin", type: "romanization", scriptCode: "Latn", languageTag: null, displayRole: "secondary", sortOrder: 1 };
      const ipa = { ...primary, id: randomUUID(), name: "IPA", type: "phonetic", scriptCode: "Latn", languageTag: null, displayRole: null, sortOrder: 2 };
      await browser.tauri.execute(
        ({ core }, request) => core.invoke("update_project_settings", { request }),
        { name: "Desktop smoke", languageName: "Traditional Chinese", languageCode: "yue", analysisLanguage: "zh-TW", description: null, writingSystems: [primary, pinyin, ipa], partOfSpeechOptions: ["Verb", "Noun"], semanticDomainOptions: ["Motion"] },
      );

      const entry = await browser.tauri.execute(({ core }) => core.invoke("create_entry")) as any;
      entry.forms = [
        { id: randomUUID(), writingSystemId: primary.id, text: "過", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 0 },
        { id: randomUUID(), writingSystemId: pinyin.id, text: "guò", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 1 },
        { id: randomUUID(), writingSystemId: ipa.id, text: "kuo˥˩", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 2 },
      ];
      entry.relations = [{ id: randomUUID(), targetEntryId: null, relationType: "root", fallbackText: "guo", notes: null, sortOrder: 0 }];
      entry.senses = [{
        id: randomUUID(), gloss: "通過", definition: null, partOfSpeech: "Verb", semanticDomain: null, sortOrder: 0,
        examples: [{
          id: randomUUID(), translation: "他過河了。", notes: "field note", sortOrder: 0,
          forms: [
            { id: randomUUID(), writingSystemId: primary.id, text: "他過河了。", sortOrder: 0 },
            { id: randomUUID(), writingSystemId: pinyin.id, text: "Tā guò hé le.", sortOrder: 1 },
            { id: randomUUID(), writingSystemId: ipa.id, text: "tʰa˥ kuo˥˩", sortOrder: 2 },
          ],
        }],
      }, {
        id: randomUUID(), gloss: "經歷", definition: null, partOfSpeech: "Verb", semanticDomain: null, sortOrder: 1,
        examples: [{ id: randomUUID(), translation: "我經歷過。", notes: null, sortOrder: 0, forms: [{ id: randomUUID(), writingSystemId: primary.id, text: "我經歷過。", sortOrder: 0 }] }],
      }];
      const saved = await browser.tauri.execute(
        ({ core }, request) => core.invoke("save_entry", { request }),
        { entry, expectedRevision: entry.revision },
      ) as any;
      const matches = await browser.tauri.execute(
        ({ core }, query) => core.invoke("query_entry_summaries", { query }),
        "guo",
      ) as any[];
      expect(matches).toHaveLength(1);

      const exportSnapshot = await browser.tauri.execute(({ core }) => core.invoke("get_project_snapshot")) as any;
      exportSnapshot.exportSettings.corpus.partOfSpeechMappings = { Verb: "verb" };
      exportSnapshot.exportSettings.latex.pronunciationWritingSystemId = ipa.id;
      await browser.tauri.execute(
        ({ core }, settings) => core.invoke("save_export_settings", { settings }),
        exportSnapshot.exportSettings,
      );
      const csvPreview = await browser.tauri.execute(
        ({ core }) => core.invoke("preview_export", { kind: "corpusCsv" }),
      ) as any;
      expect(csvPreview.rowCount).toBe(2);
      expect(csvPreview.issues.filter((issue: any) => issue.severity === "error")).toHaveLength(0);
      const csvPath = join(parentDir, "corpus.csv");
      await browser.tauri.execute(
        ({ core }, request) => core.invoke("export_project", { request }),
        { kind: "corpusCsv", destination: csvPath, snapshotToken: csvPreview.snapshotToken, overwrite: false },
      );
      expect(readFileSync(csvPath, "utf8")).toBe(
        "form,gloss_zh,word_root,example,example_translation_zh,ipa,part_of_speech,gloss_en,notes\r\n" +
        "過,通過,guo,他過河了。,他過河了。,kuo˥˩,verb,,example_notes: field note\r\n" +
        "過,經歷,guo,我經歷過。,我經歷過。,kuo˥˩,verb,,\r\n",
      );

      for (const packId of ["tex-gyre-termes", "noto-serif-cjk-tc", "noto-serif", "charis-sil"]) {
        const installed = await browser.tauri.execute(
          ({ core }, id) => core.invoke("install_font_pack", { packId: id }),
          packId,
        ) as any;
        expect(installed.state).toBe("installed");
      }

      const latexPreview = await browser.tauri.execute(
        ({ core }) => core.invoke("preview_export", { kind: "latex" }),
      ) as any;
      expect(latexPreview.issues.filter((issue: any) => issue.severity === "error")).toHaveLength(0);
      const latexResult = await browser.tauri.execute(
        ({ core }, request) => core.invoke("export_project", { request }),
        { kind: "latex", destination: parentDir, snapshotToken: latexPreview.snapshotToken, overwrite: false },
      ) as any;
      expect(readdirSync(latexResult.latexDirectory).sort()).toEqual([".latexmkrc", "README.md", "entries.tex", "fonts", "main.tex", "reverse-index.tex"]);
      expect(existsSync(join(latexResult.latexDirectory, "fonts", "tex-gyre-termes", "LICENSE.txt"))).toBe(true);
      expect(existsSync(join(latexResult.latexDirectory, "fonts", "charis-sil", "Charis-Regular.ttf"))).toBe(true);
      expect(existsSync(latexResult.zipPath)).toBe(true);

      await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      projectOpen = false;
      await browser.tauri.execute(
        ({ core }, path) => core.invoke("open_project", { path }),
        snapshot.rootPath,
      );
      projectOpen = true;
      const reopened = await browser.tauri.execute(
        ({ core }, id) => core.invoke("load_entry", { id }),
        saved.id,
      ) as any;
      expect(reopened.senses[0].examples[0].forms).toHaveLength(3);
      expect(reopened.senses[0].examples[0].translation).toBe("他過河了。");
    } finally {
      if (projectOpen) {
        await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      }
      rmSync(parentDir, { recursive: true, force: true });
    }
  });
});
