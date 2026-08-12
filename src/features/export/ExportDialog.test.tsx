import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { ProjectSnapshot } from "../../types/domain";

const { backendMock } = vi.hoisted(() => ({
  backendMock: {
    saveExportSettings: vi.fn(),
    previewExport: vi.fn(),
    exportProject: vi.fn(),
    detectXeLatex: vi.fn(),
    chooseCsvDestination: vi.fn(),
    chooseFolder: vi.fn(),
    openOverleaf: vi.fn(),
    openOverleafCompilerHelp: vi.fn(),
  },
}));
vi.mock("../../lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error { code = "export_filesystem"; },
}));

import { ExportDialog } from "./ExportDialog";

const snapshot: ProjectSnapshot = {
  rootPath: "/tmp/Test.bkuw",
  project: { id: "p1", name: "Test", languageName: null, languageCode: null, analysisLanguage: "zh-TW", description: null, createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z" },
  writingSystems: [{ id: "ws1", name: "Traditional Chinese", type: "orthography", scriptCode: "Hant", languageTag: "zh-Hant", displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null }],
  partOfSpeechOptions: ["動詞"], semanticDomainOptions: [], entries: [],
  exportSettings: { version: 1, corpus: { partOfSpeechMappings: {} }, latex: { title: "Test", author: "", headwordWritingSystemId: "ws1", pronunciationWritingSystemId: null, exampleWritingSystemId: "ws1", collationLanguageTag: "zh-Hant", sectionMode: "auto", reverseIndex: "gloss", fontPresets: { ws1: "auto" } } },
};

describe("ExportDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    backendMock.saveExportSettings.mockImplementation(async (value) => value);
    backendMock.detectXeLatex.mockResolvedValue({ available: false, path: null });
    backendMock.previewExport.mockResolvedValue({ snapshotToken: "token", rowCount: 1, issues: [], omitted: { examples: 0, exampleForms: 0, baseRelations: 0 } });
    backendMock.chooseCsvDestination.mockResolvedValue("/tmp/Test.csv");
    backendMock.exportProject.mockResolvedValue({ csvPath: "/tmp/Test.csv", latexDirectory: null, zipPath: null, pdfPath: null, pdfStatus: "notRequested", rowCount: 1, issues: [], diagnosticPath: null });
  });

  it("flushes, persists the POS mapping, previews, and exports corpus CSV", async () => {
    const flush = vi.fn(async () => undefined);
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={flush} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("動詞"), { target: { value: "verb" } });
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(flush).toHaveBeenCalled());
    expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({ corpus: { partOfSpeechMappings: { 動詞: "verb" } } }));
    expect(await screen.findByText("1 rows ready")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));
    await waitFor(() => expect(backendMock.exportProject).toHaveBeenCalledWith(expect.objectContaining({ kind: "corpusCsv", snapshotToken: "token" })));
    expect(await screen.findByText("Export complete")).toBeInTheDocument();
  });

  it("renders the missing-XeLaTeX Overleaf flow in Taiwan Traditional Chinese", async () => {
    await i18n.changeLanguage("zh-TW");
    backendMock.previewExport.mockResolvedValue({ snapshotToken: "pdf-token", rowCount: 1, issues: [], omitted: { examples: 0, exampleForms: 0, baseRelations: 0 } });
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.exportProject.mockResolvedValue({ csvPath: null, latexDirectory: "/tmp/Test-latex", zipPath: "/tmp/Test.zip", pdfPath: null, pdfStatus: "xeLatexMissing", rowCount: 1, issues: [], diagnosticPath: null });
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("PDF"));
    fireEvent.click(screen.getByRole("button", { name: "預覽" }));
    await screen.findByText("可匯出 1 列");
    fireEvent.click(screen.getByRole("button", { name: "選擇位置並匯出" }));
    expect(await screen.findByText("找不到 XeLaTeX；已建立可上傳 Overleaf 的 ZIP，但未產生 PDF。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "開啟 Overleaf" }));
    expect(backendMock.openOverleaf).toHaveBeenCalled();
  });
});
