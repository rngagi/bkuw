import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { ManualSortLayout, ProjectSnapshot } from "../../types/domain";
import { SortOrderDialog } from "./SortOrderDialog";

const snapshot = {
  entrySortSettings: { version: 1, mode: "manual", writingSystemId: "ws", alphabet: ["a", "b"] },
  manualSortLayout: { version: 1, items: [{ kind: "heading", id: "a", label: "A" }, { kind: "entry", entryId: "one" }, { kind: "heading", id: "b", label: "B" }, { kind: "entry", entryId: "two" }] },
  entries: [
    { id: "one", primaryForm: "ama", secondaryForm: null, pronunciationForm: null, pronunciationWritingSystemId: null, senses: [], revision: 1, sectionLabel: "A", manualOrderPending: false },
    { id: "two", primaryForm: "baba", secondaryForm: null, pronunciationForm: null, pronunciationWritingSystemId: null, senses: [], revision: 1, sectionLabel: "B", manualOrderPending: false },
  ],
} as unknown as ProjectSnapshot;

describe("SortOrderDialog", () => {
  beforeEach(async () => { await i18n.changeLanguage("en"); });

  it("reorders headings and entries and saves the exact layout", async () => {
    const onSave = vi.fn<(layout: ManualSortLayout) => Promise<void>>(async () => undefined);
    render(<SortOrderDialog open snapshot={snapshot} onOpenChange={vi.fn()} onSave={onSave} />);
    const moveUp = screen.getAllByRole("button", { name: "Move up" });
    fireEvent.click(moveUp[moveUp.length - 1]);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onSave).toHaveBeenCalled());
    expect(onSave.mock.calls[0]?.[0].items.map((item) => item.kind === "heading" ? item.label : item.entryId)).toEqual(["A", "one", "two", "B"]);
  });
});
