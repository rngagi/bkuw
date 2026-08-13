import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "./i18n";
import type { LexicalEntry, ProjectSnapshot } from "./types/domain";

const { backendMock } = vi.hoisted(() => ({
  backendMock: {
    chooseFolder: vi.fn(),
    createProject: vi.fn(),
    openProject: vi.fn(),
    closeProject: vi.fn(),
    updateProjectSettings: vi.fn(),
    queryEntries: vi.fn(),
    loadEntry: vi.fn(),
    createEntry: vi.fn(),
    saveEntry: vi.fn(),
    deleteEntry: vi.fn(),
    restoreEntry: vi.fn(),
    saveEntrySortSettings: vi.fn(),
    saveManualSortLayout: vi.fn(),
    saveExportSettings: vi.fn(),
    previewExport: vi.fn(),
    exportProject: vi.fn(),
    detectXeLatex: vi.fn(),
    chooseCsvDestination: vi.fn(),
    openOverleaf: vi.fn(),
    openOverleafCompilerHelp: vi.fn(),
  },
}));

vi.mock("./lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error {
    code: string;
    details?: string;
    constructor(code: string, message: string, details?: string) {
      super(message);
      this.code = code;
      this.details = details;
    }
  },
}));

import App from "./App";
import { CommandError } from "./lib/tauri";

const entry: LexicalEntry = {
  id: "entry-1",
  notes: null,
  sectionOverride: null,
  revision: 0,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  forms: [],
  senses: [],
  relations: [],
};

const snapshot: ProjectSnapshot = {
  rootPath: "/tmp/Test.bkuw",
  project: {
    id: "project-1", name: "Test", languageName: null, languageCode: null,
    analysisLanguage: "zh-TW", description: null, createdAt: entry.createdAt, updatedAt: entry.updatedAt,
  },
  writingSystems: [{
    id: "ws-1", name: "Primary orthography", type: "orthography", scriptCode: null,
    languageTag: null, displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null,
  }],
  partOfSpeechOptions: ["Verb", "Noun"],
  semanticDomainOptions: ["Motion"],
  exportSettings: {
    version: 1, corpus: { partOfSpeechMappings: {} },
    latex: { title: "Test", author: "", headwordWritingSystemId: "ws-1", pronunciationWritingSystemId: null, exampleWritingSystemId: "ws-1", collationLanguageTag: null, sectionMode: "auto", reverseIndex: "gloss", relatedEntries: "none", fontPresets: { "ws-1": "auto" } },
  },
  entrySortSettings: { version: 1, mode: "auto", writingSystemId: "ws-1", alphabet: [] },
  manualSortLayout: { version: 1, items: [] },
  entries: [],
};

describe("App keyboard and delete workflow", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    vi.stubGlobal("crypto", { randomUUID: vi.fn(() => `id-${Math.random()}`) });
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.createProject.mockResolvedValue(snapshot);
    backendMock.createEntry.mockResolvedValue(entry);
    backendMock.queryEntries.mockResolvedValue([]);
    backendMock.saveEntry.mockImplementation(async (draft: LexicalEntry) => ({ ...draft, revision: draft.revision + 1 }));
    backendMock.deleteEntry.mockResolvedValue({ id: entry.id, deletedAt: "2026-01-01T00:00:01Z" });
    backendMock.restoreEntry.mockResolvedValue({ ...entry, revision: 2 });
  });

  it("supports create, focus, add-sense, save, delete, and undo shortcuts", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose folder" }));
    await waitFor(() => expect(screen.getByDisplayValue("/tmp")).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Test" } });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(await screen.findByRole("dialog", { name: "Set up this project's writing systems" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await screen.findByRole("button", { name: "New entry" });

    fireEvent.keyDown(window, { key: "n", ctrlKey: true });
    await screen.findByRole("button", { name: "Delete entry" });
    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(await screen.findByText("Sense 1")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(screen.getByRole("textbox", { name: "Search every lexical form…" })).toHaveFocus();
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    await waitFor(() => expect(backendMock.saveEntry).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Delete entry" }));
    fireEvent.click(await screen.findByRole("button", { name: "Delete entry" }));
    await waitFor(() => expect(backendMock.deleteEntry).toHaveBeenCalled());
    fireEvent.click(await screen.findByRole("button", { name: "Undo" }));
    await waitFor(() => expect(backendMock.restoreEntry).toHaveBeenCalledWith(entry.id));

    fireEvent.click(screen.getByRole("button", { name: "Close project" }));
    await screen.findByRole("heading", { name: "Your lexical projects, stored on this device" });
    expect(backendMock.closeProject).toHaveBeenCalled();
  });

  it("shows a modal when a project name already exists", async () => {
    backendMock.createProject.mockRejectedValue(new CommandError("project_exists", "exists"));
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose folder" }));
    await waitFor(() => expect(screen.getByDisplayValue("/tmp")).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Test" } });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(await screen.findByRole("alertdialog", { name: "Project name already exists" })).toBeInTheDocument();
  });

  it("shows Saved as soon as the entry save succeeds without waiting for list refresh", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose folder" }));
    await waitFor(() => expect(screen.getByDisplayValue("/tmp")).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Test" } });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    const onboardingDialog = await screen.findByRole("dialog", { name: "Set up this project's writing systems" });
    fireEvent.click(within(onboardingDialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Set up this project's writing systems" })).not.toBeInTheDocument());
    fireEvent.click(await screen.findByRole("button", { name: "New entry" }));
    const text = await screen.findByLabelText("Text");
    fireEvent.change(text, { target: { value: "中" } });
    backendMock.queryEntries.mockImplementation(() => new Promise(() => undefined));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(backendMock.saveEntry).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByText("Saved")).toBeInTheDocument(), { timeout: 500 });
  });

  it("recovers a manual-mode project whose layout was never initialized", async () => {
    const manualSnapshot: ProjectSnapshot = {
      ...snapshot,
      entrySortSettings: { version: 1, mode: "manual", writingSystemId: "ws-1", alphabet: ["a", "ng"] },
      entries: [{ id: "entry-1", primaryForm: "ngayan", secondaryForm: null, pronunciationForm: null, pronunciationWritingSystemId: null, senses: [], revision: 1, sectionLabel: "NG", manualOrderPending: true }],
    };
    backendMock.createProject.mockResolvedValue(manualSnapshot);
    backendMock.queryEntries.mockResolvedValue(manualSnapshot.entries);
    backendMock.saveManualSortLayout.mockImplementation(async (layout: ProjectSnapshot["manualSortLayout"]) => ({
      ...manualSnapshot,
      manualSortLayout: layout,
      entries: manualSnapshot.entries.map((item) => ({ ...item, manualOrderPending: false })),
    }));
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose folder" }));
    await waitFor(() => expect(screen.getByDisplayValue("/tmp")).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Test" } });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    const onboardingDialog = await screen.findByRole("dialog", { name: "Set up this project's writing systems" });
    fireEvent.click(within(onboardingDialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(backendMock.queryEntries).toHaveBeenCalled());

    fireEvent.click(await screen.findByRole("button", { name: "Arrange manual order" }));
    const sortDialog = await screen.findByRole("dialog", { name: "Arrange entries and headings" });
    expect(within(sortDialog).getByText("ngayan")).toBeInTheDocument();
    expect(within(sortDialog).getByText(/No manual layout has been saved yet/)).toBeInTheDocument();
    fireEvent.click(within(sortDialog).getByRole("button", { name: "Save" }));

    await waitFor(() => expect(backendMock.saveManualSortLayout).toHaveBeenCalledWith({
      version: 1,
      items: [{ kind: "entry", entryId: "entry-1" }],
    }));
  });
});
