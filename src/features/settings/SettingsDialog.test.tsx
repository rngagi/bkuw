import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { ProjectSnapshot, WritingSystem } from "../../types/domain";
import { SettingsDialog } from "./SettingsDialog";

const snapshot: ProjectSnapshot = {
  rootPath: "/tmp/Test.bkuw",
  project: {
    id: "project-1",
    name: "Test",
    languageName: null,
    languageCode: null,
    analysisLanguage: null,
    description: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  writingSystems: [{
    id: "ws-1",
    name: "Traditional Chinese",
    type: "orthography",
    scriptCode: "Hant",
    languageTag: "zh-Hant",
    displayRole: "primary",
    sortOrder: 0,
    fontFamily: null,
    notes: null,
  }],
  partOfSpeechOptions: [],
  semanticDomainOptions: [],
  exportSettings: {
    version: 1, corpus: { partOfSpeechMappings: {} },
    latex: { title: "Test", author: "", headwordWritingSystemId: "ws-1", pronunciationWritingSystemId: null, exampleWritingSystemId: "ws-1", collationLanguageTag: null, sectionMode: "auto", reverseIndex: "gloss", relatedEntries: "none", includeSenseImages: false, fontPresets: { "ws-1": "auto" } },
  },
  entrySortSettings: { version: 1, mode: "auto", writingSystemId: "ws-1", alphabet: [] },
  manualSortLayout: { version: 1, items: [] },
  entries: [],
};

describe("SettingsDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.stubGlobal("crypto", { randomUUID: vi.fn(() => "ws-2") });
  });

  it("validates display roles and saves dynamic writing-system metadata", async () => {
    const onSave = vi.fn(async (_request: { writingSystems: WritingSystem[]; partOfSpeechOptions: string[]; semanticDomainOptions: string[] }) => undefined);
    render(<SettingsDialog open snapshot={snapshot} onOpenChange={vi.fn()} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText("Display role"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Check the highlighted project data");
    expect(onSave).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Display role"), { target: { value: "primary" } });
    fireEvent.click(screen.getByRole("button", { name: "Add writing system" }));
    const names = screen.getAllByLabelText("Name");
    const roles = screen.getAllByLabelText("Display role");
    fireEvent.click(screen.getAllByText("Optional technical details")[1]);
    const scripts = screen.getAllByLabelText("Script code");
    const languages = screen.getAllByLabelText("Language tag");
    const fonts = screen.getAllByLabelText("Font family");
    fireEvent.change(names[1], { target: { value: "Pinyin" } });
    fireEvent.change(roles[1], { target: { value: "secondary" } });
    fireEvent.change(scripts[1], { target: { value: "Latn" } });
    fireEvent.change(languages[1], { target: { value: "zh-Latn-pinyin" } });
    fireEvent.change(fonts[1], { target: { value: "Noto Sans" } });
    fireEvent.change(screen.getByLabelText("Parts of speech"), { target: { value: "Verb" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add option" })[0]);
    fireEvent.change(screen.getByLabelText("Semantic domains"), { target: { value: "Motion" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Add option" })[1]);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    const systems = onSave.mock.calls[0][0].writingSystems;
    expect(systems).toHaveLength(2);
    expect(systems[1]).toMatchObject({
      name: "Pinyin",
      displayRole: "secondary",
      scriptCode: "Latn",
      languageTag: "zh-Latn-pinyin",
      fontFamily: "Noto Sans",
      sortOrder: 1,
    });
    expect(onSave.mock.calls[0][0].partOfSpeechOptions).toEqual(["Verb"]);
    expect(onSave.mock.calls[0][0].semanticDomainOptions).toEqual(["Motion"]);
  });
});
