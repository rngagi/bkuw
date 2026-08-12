import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { EntrySummary, LexicalEntry, WritingSystem } from "../../types/domain";
import { EntryEditor, type EntryEditorHandle } from "./EntryEditor";
import { createRef } from "react";

const writingSystems: WritingSystem[] = [
  { id: "ws-native", name: "Traditional Chinese", type: "orthography", scriptCode: "Hant", languageTag: "zh-Hant", displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null },
  { id: "ws-pinyin", name: "Pinyin", type: "romanization", scriptCode: "Latn", languageTag: null, displayRole: "secondary", sortOrder: 1, fontFamily: null, notes: null },
];

function emptyEntry(): LexicalEntry {
  return { id: "entry-1", notes: null, revision: 0, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z", forms: [{ id: "form-1", writingSystemId: "ws-native", text: "", variantLabel: null, dialect: null, status: null, notes: null, sortOrder: 0 }], senses: [], relations: [] };
}

const metadataProps = { partOfSpeechOptions: ["Verb", "Noun"], semanticDomainOptions: ["Motion"] };

describe("EntryEditor", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.stubGlobal("crypto", { randomUUID: vi.fn(() => `id-${Math.random()}`) });
  });

  it("saves nested multi-writing-system examples as one aggregate", async () => {
    const onSave = vi.fn(async (draft: LexicalEntry) => ({ ...draft, revision: draft.revision + 1 }));
    const ref = createRef<EntryEditorHandle>();
    render(<EntryEditor {...metadataProps} ref={ref} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: /add sense/i }));
    fireEvent.change(screen.getByLabelText("Gloss"), { target: { value: "cross" } });
    fireEvent.change(screen.getByLabelText("Part of speech"), { target: { value: "Verb" } });
    fireEvent.click(screen.getByRole("button", { name: /add example/i }));
    fireEvent.click(screen.getByRole("button", { name: /add example form/i }));
    fireEvent.change(screen.getAllByLabelText("Text")[1], { target: { value: "他過河了。" } });
    fireEvent.change(screen.getByLabelText("Translation"), { target: { value: "He crossed the river." } });

    await act(async () => { await ref.current?.flush(); });
    const saved = onSave.mock.calls[onSave.mock.calls.length - 1]?.[0];
    expect(saved?.senses[0].partOfSpeech).toBe("Verb");
    expect(saved?.senses[0].examples[0].forms[0].text).toBe("他過河了。");
    expect(saved?.senses[0].examples[0].translation).toBe("He crossed the river.");
  });

  it("keeps a dirty draft and rejects flush when the backend save fails", async () => {
    const onSave = vi.fn(async () => { throw new Error("disk full"); });
    const ref = createRef<EntryEditorHandle>();
    render(<EntryEditor {...metadataProps} ref={ref} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);
    fireEvent.change(screen.getAllByLabelText("Text")[0], { target: { value: "guò" } });
    let failure: unknown;
    await act(async () => {
      try { await ref.current?.flush(); } catch (error) { failure = error; }
    });
    expect(failure).toEqual(new Error("disk full"));
    expect(screen.getAllByLabelText("Text")[0]).toHaveValue("guò");
    expect(screen.getByText("Save failed")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("disk full");
  });

  it("normalizes nested sort order before saving", async () => {
    const entry = emptyEntry();
    entry.senses = [{
      id: "sense-1", gloss: null, definition: null, partOfSpeech: null,
      semanticDomain: null, sortOrder: 0,
      examples: [
        { id: "example-a", translation: "A", notes: null, sortOrder: 0, forms: [] },
        { id: "example-b", translation: "B", notes: null, sortOrder: 1, forms: [] },
      ],
    }];
    const onSave = vi.fn(async (draft: LexicalEntry) => ({ ...draft, revision: 1 }));
    const ref = createRef<EntryEditorHandle>();
    render(<EntryEditor {...metadataProps} ref={ref} entry={entry} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);

    const enabledMoveDown = screen.getAllByRole("button", { name: "Move down" }).find((button) => !button.hasAttribute("disabled"));
    fireEvent.click(enabledMoveDown as HTMLButtonElement);
    await act(async () => { await ref.current?.flush(); });

    const saved = onSave.mock.calls.at(-1)?.[0];
    expect(saved?.senses[0].examples.map((example) => example.translation)).toEqual(["B", "A"]);
    expect(saved?.senses[0].examples.map((example) => example.sortOrder)).toEqual([0, 1]);
  });

  it("autocompletes a linked relation and supports navigation", async () => {
    const target: EntrySummary = { id: "target-entry-1234", primaryForm: "ambuk", secondaryForm: null, partsOfSpeech: ["Noun"], revision: 1 };
    const onSave = vi.fn(async (draft: LexicalEntry) => ({ ...draft, revision: 1 }));
    const onNavigate = vi.fn();
    const ref = createRef<EntryEditorHandle>();
    render(<EntryEditor {...metadataProps} ref={ref} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[target]} onSave={onSave} onDelete={vi.fn()} onNavigate={onNavigate} />);

    fireEvent.click(screen.getByRole("button", { name: "Add relation" }));
    fireEvent.change(screen.getByLabelText("Linked entry"), { target: { value: "ambuk — target-e" } });
    await act(async () => { await ref.current?.flush(); });
    expect(onSave.mock.calls.at(-1)?.[0].relations[0].targetEntryId).toBe(target.id);

    fireEvent.click(screen.getByRole("button", { name: "Open linked entry" }));
    expect(onNavigate).toHaveBeenCalledWith(target.id);
  });

  it("autosaves a valid nested draft after the debounce", async () => {
    vi.useFakeTimers();
    try {
      const onSave = vi.fn(async (draft: LexicalEntry) => ({ ...draft, revision: 1 }));
      render(<EntryEditor {...metadataProps} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);
      expect(screen.queryByRole("button", { name: "Add form" })).not.toBeInTheDocument();
      fireEvent.change(screen.getAllByLabelText("Text")[0], { target: { value: "guò" } });

      await act(async () => {
        vi.advanceTimersByTime(701);
        await Promise.resolve();
      });
      expect(onSave).toHaveBeenCalled();
      expect(onSave.mock.calls.at(-1)?.[0].forms[0].text).toBe("guò");
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps the same field focused with its cursor after autosave completes", async () => {
    vi.useFakeTimers();
    try {
      const entry = emptyEntry();
      entry.senses = [{
        id: "sense-1",
        gloss: "",
        definition: null,
        partOfSpeech: null,
        semanticDomain: null,
        sortOrder: 0,
        examples: [],
      }];
      const onSave = vi.fn(async (draft: LexicalEntry) => ({
        ...draft,
        revision: draft.revision + 1,
        updatedAt: "2026-01-01T00:00:01Z",
      }));
      render(<EntryEditor {...metadataProps} entry={entry} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);
      const gloss = screen.getByLabelText("Gloss") as HTMLInputElement;
      gloss.focus();
      fireEvent.change(gloss, { target: { value: "cross river" } });
      gloss.setSelectionRange(5, 5);

      await act(async () => {
        vi.advanceTimersByTime(701);
        await Promise.resolve();
      });

      expect(onSave).toHaveBeenCalledTimes(1);
      expect(screen.getByLabelText("Gloss")).toBe(gloss);
      expect(gloss).toHaveFocus();
      expect(gloss.selectionStart).toBe(5);
      expect(gloss.selectionEnd).toBe(5);
      expect(screen.getByRole("status")).toHaveTextContent("Saved");
    } finally {
      vi.useRealTimers();
    }
  });

  it("creates an example with the primary form and limits forms to configured systems", () => {
    render(<EntryEditor {...metadataProps} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={vi.fn()} onDelete={vi.fn()} onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Add sense" }));
    fireEvent.click(screen.getByRole("button", { name: "Add example" }));
    expect(screen.getAllByText("Traditional Chinese").length).toBeGreaterThan(0);
    const addForm = screen.getByRole("button", { name: "Add example form" });
    expect(addForm).toBeEnabled();
    fireEvent.click(addForm);
    expect(screen.getByRole("button", { name: "Add example form" })).toBeDisabled();
  });

  it("requires delete confirmation", async () => {
    const onDelete = vi.fn(async () => undefined);
    render(<EntryEditor {...metadataProps} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={vi.fn()} onDelete={onDelete} onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Delete entry" }));
    expect(onDelete).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("alertdialog").querySelector("button[type=button]:last-child") as HTMLButtonElement);
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it("does not autosave or reset the field while an IME composition is active", async () => {
    vi.useFakeTimers();
    try {
      const onSave = vi.fn(async (draft: LexicalEntry) => ({ ...draft, revision: 1 }));
      render(<EntryEditor {...metadataProps} entry={emptyEntry()} writingSystems={writingSystems} entryOptions={[]} onSave={onSave} onDelete={vi.fn()} onNavigate={vi.fn()} />);
      const primary = screen.getAllByLabelText("Text")[0];
      primary.focus();
      fireEvent.compositionStart(primary);
      fireEvent.change(primary, { target: { value: "ㄓ" } });
      await act(async () => { vi.advanceTimersByTime(1_000); });

      expect(onSave).not.toHaveBeenCalled();
      expect(primary).toHaveValue("ㄓ");
      expect(primary).toHaveFocus();

      fireEvent.change(primary, { target: { value: "中" } });
      fireEvent.compositionEnd(primary);
      await act(async () => { vi.advanceTimersByTime(701); await Promise.resolve(); });
      expect(onSave).toHaveBeenCalledTimes(1);
      expect(onSave.mock.calls[0][0].forms[0].text).toBe("中");
    } finally {
      vi.useRealTimers();
    }
  });
});
