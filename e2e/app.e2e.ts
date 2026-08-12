import { mkdtempSync, rmSync } from "node:fs";
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
      await browser.tauri.execute(
        ({ core }, request) => core.invoke("update_project_settings", { request }),
        { name: "Desktop smoke", languageName: "Traditional Chinese", languageCode: "yue", description: null, writingSystems: [primary, pinyin, ipa], partOfSpeechOptions: ["Verb", "Noun"], semanticDomainOptions: ["Motion"] },
      );

      const entry = await browser.tauri.execute(({ core }) => core.invoke("create_entry")) as any;
      entry.forms = [
        { id: randomUUID(), writingSystemId: primary.id, text: "過", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 0 },
        { id: randomUUID(), writingSystemId: pinyin.id, text: "guò", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 1 },
        { id: randomUUID(), writingSystemId: ipa.id, text: "kuo˥˩", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 2 },
      ];
      entry.senses = [{
        id: randomUUID(), gloss: "cross", definition: null, partOfSpeech: "Verb", semanticDomain: null, sortOrder: 0,
        examples: [{
          id: randomUUID(), translation: "He crossed the river.", notes: "field note", sortOrder: 0,
          forms: [
            { id: randomUUID(), writingSystemId: primary.id, text: "他過河了。", sortOrder: 0 },
            { id: randomUUID(), writingSystemId: pinyin.id, text: "Tā guò hé le.", sortOrder: 1 },
            { id: randomUUID(), writingSystemId: ipa.id, text: "tʰa˥ kuo˥˩", sortOrder: 2 },
          ],
        }],
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
      expect(reopened.senses[0].examples[0].translation).toBe("He crossed the river.");
    } finally {
      if (projectOpen) {
        await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      }
      rmSync(parentDir, { recursive: true, force: true });
    }
  });
});
