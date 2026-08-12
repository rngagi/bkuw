import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { $, browser, expect } from "@wdio/globals";

describe("bkuw desktop shell", () => {
  it("renders the detected locale and opens the project dialog", async () => {
    const heading = await $("h1");
    await expect(heading).toBeDisplayed();
    const chinese = (await heading.getText()).includes("詞彙專案");
    await $(chinese ? "button=建立專案" : "button=Create project").click();
    await expect($("[role=dialog]")).toBeDisplayed();
    await expect($("[role=dialog] h2")).toHaveText(chinese ? "建立專案" : "Create project");
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
      const tibetan = { ...primary, id: randomUUID(), name: "Tibetan", type: "orthography", scriptCode: "Tibt", languageTag: "bo", displayRole: null, sortOrder: 3 };
      await browser.tauri.execute(
        ({ core }, request) => core.invoke("update_project_settings", { request }),
        { name: "Desktop smoke", languageName: "Traditional Chinese", languageCode: "yue", analysisLanguage: "zh-TW", description: null, writingSystems: [primary, pinyin, ipa, tibetan], partOfSpeechOptions: ["Verb", "Noun"], semanticDomainOptions: ["Motion"] },
      );

      const entry = await browser.tauri.execute(({ core }) => core.invoke("create_entry")) as any;
      entry.forms = [
        { id: randomUUID(), writingSystemId: primary.id, text: "過", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 0 },
        { id: randomUUID(), writingSystemId: pinyin.id, text: "guò", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 1 },
        { id: randomUUID(), writingSystemId: ipa.id, text: "kuo˥˩", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 2 },
        { id: randomUUID(), writingSystemId: tibetan.id, text: "འགྲོ", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 3 },
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
            { id: randomUUID(), writingSystemId: tibetan.id, text: "ཁོང་ཆུ་བོ་བརྒལ།", sortOrder: 3 },
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

      for (const packId of ["tex-gyre-termes", "noto-serif-cjk-tc", "noto-serif", "charis-sil", "noto-serif-tibetan"]) {
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
      expect(reopened.senses[0].examples[0].forms).toHaveLength(4);
      expect(reopened.senses[0].examples[0].translation).toBe("他過河了。");
    } finally {
      if (projectOpen) {
        await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      }
      rmSync(parentDir, { recursive: true, force: true });
    }
  });
});
