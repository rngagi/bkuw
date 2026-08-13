import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
    listFontPacks: vi.fn(),
    installFontPack: vi.fn(),
  },
}));
vi.mock("../../lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error {
    constructor(public code: string, message: string, public details?: string) {
      super(message);
    }
  },
}));

import { CommandError } from "../../lib/tauri";
import { ExportDialog } from "./ExportDialog";

const snapshot: ProjectSnapshot = {
  rootPath: "/tmp/Test.bkuw",
  project: { id: "p1", name: "Test", languageName: null, languageCode: null, analysisLanguage: "zh-TW", description: null, createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z" },
  writingSystems: [{ id: "ws1", name: "Traditional Chinese", type: "orthography", scriptCode: "Hant", languageTag: "zh-Hant", displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null }],
  partOfSpeechOptions: ["動詞"], semanticDomainOptions: [], entries: [],
  exportSettings: { version: 1, corpus: { partOfSpeechMappings: {} }, latex: { title: "Test", author: "", headwordWritingSystemId: "ws1", pronunciationWritingSystemId: null, exampleWritingSystemId: "ws1", collationLanguageTag: "zh-Hant", sectionMode: "auto", reverseIndex: "gloss", relatedEntries: "none", includeSenseImages: false, fontPresets: { ws1: "auto" } } },
  entrySortSettings: { version: 1, mode: "auto", writingSystemId: "ws1", alphabet: [] },
  manualSortLayout: { version: 1, items: [] },
};

describe("ExportDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    backendMock.saveExportSettings.mockImplementation(async (value) => value);
    backendMock.detectXeLatex.mockResolvedValue({ available: false, path: null });
    backendMock.listFontPacks.mockResolvedValue([
      { id: "tex-gyre-termes", version: "2.004", state: "missing", mandatory: true, installedBytes: 0 },
      { id: "noto-serif-cjk-tc", version: "2.003", state: "installed", mandatory: false, installedBytes: 1 },
    ]);
    backendMock.installFontPack.mockResolvedValue({ id: "tex-gyre-termes", version: "2.004", state: "installed", mandatory: true, installedBytes: 1 });
    backendMock.previewExport.mockResolvedValue({ snapshotToken: "token", rowCount: 1, issues: [], omitted: { examples: 0, exampleForms: 0, baseRelations: 0 }, requiredFontPacks: [] });
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

  it("persists the optional sense-photo setting for LaTeX exports", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));
    fireEvent.click(screen.getByLabelText("Include sense photos"));
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(
      expect.objectContaining({ latex: expect.objectContaining({ includeSenseImages: true }) }),
    ));
  });

  it("shows meaningful progress while PDF generation runs in the background", async () => {
    let finishExport!: (value: unknown) => void;
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.exportProject.mockReturnValue(new Promise((resolve) => { finishExport = resolve; }));
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("PDF"));
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await screen.findByText("1 rows ready");
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));

    expect(await screen.findByRole("progressbar", { name: "Generating sources and compiling the PDF…" })).toBeInTheDocument();
    expect(screen.getByText("XeLaTeX runs in the background and may take up to two minutes.")).toBeInTheDocument();

    finishExport({ csvPath: null, latexDirectory: "/tmp/Test-latex", zipPath: "/tmp/Test.zip", pdfPath: "/tmp/Test.pdf", pdfStatus: "created", rowCount: 1, issues: [], diagnosticPath: null });
    expect(await screen.findByText("Export complete")).toBeInTheDocument();
  });

  it("defers font and XeLaTeX checks until a TeX format is selected", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    expect(backendMock.listFontPacks).not.toHaveBeenCalled();
    expect(backendMock.detectXeLatex).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("LaTeX"));
    await waitFor(() => expect(backendMock.listFontPacks).toHaveBeenCalledTimes(1));
    expect(backendMock.detectXeLatex).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("PDF"));
    await waitFor(() => expect(backendMock.detectXeLatex).toHaveBeenCalledTimes(1));
  });

  it("renders the missing-XeLaTeX Overleaf flow in Taiwan Traditional Chinese", async () => {
    await i18n.changeLanguage("zh-TW");
    backendMock.previewExport.mockResolvedValue({ snapshotToken: "pdf-token", rowCount: 1, issues: [], omitted: { examples: 0, exampleForms: 0, baseRelations: 0 }, requiredFontPacks: [] });
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

  it("persists the optional direct related-entry mode for LaTeX", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));
    fireEvent.change(screen.getByLabelText("Related entries"), { target: { value: "both" } });
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({ latex: expect.objectContaining({ relatedEntries: "both" }) })));
  });

  it("prevents the headword writing system from being selected again as pronunciation", async () => {
    const ipaSnapshot: ProjectSnapshot = {
      ...snapshot,
      writingSystems: [
        snapshot.writingSystems[0],
        { id: "ws2", name: "IPA", type: "phonetic", scriptCode: "Latn", languageTag: null, displayRole: null, sortOrder: 1, fontFamily: null, notes: null },
      ],
      exportSettings: {
        ...snapshot.exportSettings,
        latex: {
          ...snapshot.exportSettings.latex,
          pronunciationWritingSystemId: "ws2",
          fontPresets: { ws1: "auto", ws2: "charisSil" },
        },
      },
    };
    render(<ExportDialog open snapshot={ipaSnapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));

    const pronunciation = screen.getByLabelText("Pronunciation writing system");
    expect(within(pronunciation).queryByRole("option", { name: "Traditional Chinese" })).not.toBeInTheDocument();
    expect(within(pronunciation).getByRole("option", { name: "IPA" })).toBeInTheDocument();
    expect(screen.getByText("Shown beside the headword and cannot use the same writing system.")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Headword writing system"), { target: { value: "ws2" } });
    expect(screen.getByLabelText("Pronunciation writing system")).toHaveValue("");
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({
      latex: expect.objectContaining({ headwordWritingSystemId: "ws2", pronunciationWritingSystemId: null }),
    })));
  });

  it("shows the preserved XeLaTeX diagnostic log path after a Windows compile failure", async () => {
    const diagnosticPath = "C:\\Users\\researcher\\Documents\\Test-latex\\diagnostic.log";
    backendMock.chooseFolder.mockResolvedValue("C:\\Users\\researcher\\Documents");
    backendMock.exportProject.mockRejectedValue(new CommandError(
      "latex_compile",
      "XeLaTeX exited with status 1",
      diagnosticPath,
    ));

    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("PDF"));
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await screen.findByText("1 rows ready");
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));

    expect(await screen.findByText("Diagnostic log location")).toBeInTheDocument();
    expect(screen.getByText(diagnosticPath)).toBeInTheDocument();
  });

  it("downloads a missing mandatory font pack and retries the LaTeX preview", async () => {
    backendMock.previewExport
      .mockResolvedValueOnce({
        snapshotToken: "blocked",
        rowCount: 1,
        issues: [{ severity: "error", code: "latex.font_pack_missing", entryId: null, senseId: null, field: "fontPacks", details: "tex-gyre-termes" }],
        omitted: { examples: 0, exampleForms: 0, baseRelations: 0 },
        requiredFontPacks: [{ id: "tex-gyre-termes", version: "2.004", state: "missing", mandatory: true, installedBytes: 0 }],
      })
      .mockResolvedValueOnce({
        snapshotToken: "ready",
        rowCount: 1,
        issues: [],
        omitted: { examples: 0, exampleForms: 0, baseRelations: 0 },
        requiredFontPacks: [{ id: "tex-gyre-termes", version: "2.004", state: "installed", mandatory: true, installedBytes: 1 }],
      });

    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(await screen.findByText("TeX Gyre Termes is required; LaTeX/PDF export is blocked until it is installed.")) .toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Download and retry" }));
    await waitFor(() => expect(backendMock.installFontPack).toHaveBeenCalledWith("tex-gyre-termes"));
    await waitFor(() => expect(backendMock.previewExport).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("0 blocking errors · 0 warnings")).toBeInTheDocument();
  });

  it("shows Charis SIL as fixed for phonemic and phonetic writing systems", async () => {
    const ipaSnapshot: ProjectSnapshot = {
      ...snapshot,
      writingSystems: [{ ...snapshot.writingSystems[0], name: "IPA", type: "phonetic", scriptCode: "Latn" }],
    };
    render(<ExportDialog open snapshot={ipaSnapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));
    expect(screen.getByText("Charis SIL (fixed for IPA)")).toBeInTheDocument();
  });

  it("offers both Chiron families and explains their typeface styles", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("LaTeX"));

    const font = screen.getByLabelText("Portable font for Traditional Chinese");
    expect(within(font).getByRole("option", { name: "Chiron Sung HK" })).toBeInTheDocument();
    expect(within(font).getByRole("option", { name: "Chiron Hei HK" })).toBeInTheDocument();

    fireEvent.change(font, { target: { value: "chironSungHk" } });
    expect(screen.getByText("Ming/Song style for Traditional Chinese, including Hong Kong glyph conventions.")).toBeInTheDocument();
    fireEvent.change(font, { target: { value: "chironHeiHk" } });
    expect(screen.getByText("Hei/sans-serif style for Traditional Chinese, including Hong Kong glyph conventions.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({
      latex: expect.objectContaining({ fontPresets: { ws1: "chironHeiHk" } }),
    })));
  });
});
