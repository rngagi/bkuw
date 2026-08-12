import { describe, expect, it } from "vitest";
import { z } from "zod";
import chinese from "../fixtures/chinese.json";
import latin from "../fixtures/latin.json";
import tibetan from "../fixtures/tibetan.json";

const fixtureSchema = z.object({
  project: z.string().min(1),
  writingSystems: z.array(z.string().min(1)).min(1),
  entries: z.array(z.object({
    forms: z.array(z.string().min(1)).min(1),
    senses: z.array(z.object({
      partOfSpeech: z.string().min(1),
      gloss: z.string().min(1),
      definition: z.string().optional(),
      examples: z.array(z.object({
        forms: z.array(z.string().min(1)).min(1),
        translation: z.string().min(1),
      })),
    })).min(1),
  })).min(1),
});

describe("demo fixtures", () => {
  it("covers Chinese, Tibetan, and Latin-script dynamic writing systems", () => {
    const fixtures = [chinese, tibetan, latin].map((fixture) => fixtureSchema.parse(fixture));
    expect(fixtures.map((fixture) => fixture.writingSystems)).toEqual([
      ["Traditional Chinese", "Pinyin", "IPA"],
      ["Tibetan", "Wylie", "IPA"],
      ["Practical orthography", "IPA"],
    ]);
    expect(fixtures[0].entries[0].senses[0].examples[0].forms).toHaveLength(3);
  });
});
