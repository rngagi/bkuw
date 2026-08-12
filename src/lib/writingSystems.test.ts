import { describe, expect, it } from "vitest";
import type { WritingSystem } from "../types/domain";
import { displayWritingSystemText } from "./writingSystems";

function system(type: WritingSystem["type"]): WritingSystem {
  return { id: type, name: type, type, scriptCode: null, languageTag: null, displayRole: null, sortOrder: 0, fontFamily: null, notes: null };
}

describe("writing-system display", () => {
  it("adds conventional delimiters without changing stored text", () => {
    expect(displayWritingSystemText("pʰøː", system("phonemic"))).toBe("/pʰøː/");
    expect(displayWritingSystemText("pʰøː", system("phonetic"))).toBe("[pʰøː]");
    expect(displayWritingSystemText("bod", system("transliteration"))).toBe("bod");
  });
});
