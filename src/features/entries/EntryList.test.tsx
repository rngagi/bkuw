import { render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { EntrySummary, WritingSystem } from "../../types/domain";
import { EntryList } from "./EntryList";

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count, estimateSize }: { count: number; estimateSize(index: number): number }) => ({
    getTotalSize: () => Array.from({ length: count }, (_, index) => estimateSize(index)).reduce((sum, size) => sum + size, 0),
    getVirtualItems: () => Array.from({ length: count }, (_, index) => ({ index, start: index * 100, size: estimateSize(index) })),
  }),
}));

const writingSystems: WritingSystem[] = [
  { id: "orth", name: "Orthography", type: "orthography", scriptCode: "Latn", languageTag: null, displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null },
  { id: "ipa", name: "IPA", type: "phonemic", scriptCode: "Latn", languageTag: null, displayRole: "secondary", sortOrder: 1, fontFamily: null, notes: null },
];

const entry: EntrySummary = {
  id: "entry-1",
  primaryForm: "ata",
  secondaryForm: "ata",
  pronunciationForm: "ata",
  pronunciationWritingSystemId: "ipa",
  senses: [
    { partOfSpeech: "名詞", gloss: "父親" },
    { partOfSpeech: "動詞", gloss: "稱作父親" },
    { partOfSpeech: "動詞", gloss: "敬稱" },
    { partOfSpeech: null, gloss: null },
  ],
  revision: 1,
  sectionLabel: "A",
  manualOrderPending: false,
};

describe("EntryList", () => {
  beforeEach(async () => { await i18n.changeLanguage("zh-TW"); });

  it("keeps IPA with the headword and preserves each sense's POS/gloss pairing", () => {
    render(<EntryList entries={[entry]} writingSystems={writingSystems} selectedId={null} hasQuery={false} onSelect={vi.fn()} />);

    const item = screen.getByRole("button");
    const headword = item.querySelector(".entry-list-headword");
    expect(headword).not.toBeNull();
    expect(within(headword as HTMLElement).getByText("ata", { selector: "strong" })).toBeInTheDocument();
    expect(within(headword as HTMLElement).getByText("/ata/")).toBeInTheDocument();
    expect(item.querySelectorAll(".entry-list-headword + span")).toHaveLength(0);

    const summaries = item.querySelectorAll(".entry-sense-summary");
    expect(summaries).toHaveLength(2);
    expect(summaries[0]).toHaveTextContent("名詞父親");
    expect(summaries[1]).toHaveTextContent("動詞稱作父親…等 3 個語義");
    expect(item).not.toHaveTextContent("敬稱");
  });
});
