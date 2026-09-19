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
    checkLatexEnvironment: vi.fn(),
    installLatex: vi.fn(),
    cancelLatexDownload: vi.fn(),
    saveLatexInstallGuide: vi.fn(),
    openExportHelp: vi.fn(),
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
  exportSettings: { version: 1, corpus: { partOfSpeechMappings: {} }, latex: { title: "Test", author: "", headwordWritingSystemId: "ws1", pronunciationWritingSystemId: null, exampleWritingSystemId: "ws1", collationLanguageTag: "zh-Hant", sectionMode: "auto", reverseIndex: "gloss", relatedEntries: "none", includeSenseImages: false, includeSemanticDomains: true, fontPresets: { ws1: "auto" } } },
  entrySortSettings: { version: 2, mode: "auto", source: "writingSystem", writingSystemId: "ws1", alphabet: [] },
  manualSortLayout: { version: 1, items: [] },
};

function choose(format: "PDF" | "LaTeX" | "CSV" | "Overleaf") {
  fireEvent.click(screen.getByLabelText(format));
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}
async function confirm() {
  await waitFor(() => expect(screen.getByRole("button", { name: "Next" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

describe("ExportDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.resetAllMocks();
    backendMock.checkLatexEnvironment.mockResolvedValue({ state: "ready", path: "/tex/xelatex", version: "XeTeX", missingFiles: [], diagnosticPath: null });
    backendMock.openOverleaf.mockResolvedValue(undefined);
    backendMock.openExportHelp.mockResolvedValue(undefined);
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
    choose("CSV");
    fireEvent.change(screen.getByLabelText("動詞"), { target: { value: "verb" } });
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await waitFor(() => expect(flush).toHaveBeenCalled());
    expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({ corpus: { partOfSpeechMappings: { 動詞: "verb" } } }));
    expect(await screen.findByText("1 rows ready")).toBeInTheDocument();
    await confirm();
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));
    await waitFor(() => expect(backendMock.exportProject).toHaveBeenCalledWith(expect.objectContaining({ kind: "corpusCsv", snapshotToken: "token" })));
    expect(await screen.findByText("Export complete")).toBeInTheDocument();
  });

  it("persists the optional sense-photo setting for LaTeX exports", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    fireEvent.click(screen.getByLabelText("Include sense photos"));
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(
      expect.objectContaining({ latex: expect.objectContaining({ includeSenseImages: true }) }),
    ));
  });

  it("lets users hide per-sense semantic domains", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    fireEvent.click(screen.getByLabelText("Show semantic domains for each sense"));
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(
      expect.objectContaining({ latex: expect.objectContaining({ includeSemanticDomains: false }) }),
    ));
  });

  it("forces per-sense semantic domains off while semantic-domain grouping is active", async () => {
    const grouped = { ...snapshot, entrySortSettings: { ...snapshot.entrySortSettings, source: "semanticDomain" as const } };
    render(<ExportDialog open snapshot={grouped} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    const checkbox = screen.getByLabelText("Show semantic domains for each sense");
    expect(checkbox).toBeDisabled();
    expect(checkbox).not.toBeChecked();
    expect(screen.getByText(/section heading already identifies the category/i)).toBeInTheDocument();
  });

  it("shows meaningful progress while PDF generation runs in the background", async () => {
    let finishExport!: (value: unknown) => void;
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.exportProject.mockReturnValue(new Promise((resolve) => { finishExport = resolve; }));
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    await confirm();
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));

    expect(await screen.findByRole("progressbar", { name: "Generating sources and compiling the PDF…" })).toBeInTheDocument();
    expect(screen.getByText("XeLaTeX runs in the background and may take up to two minutes.")).toBeInTheDocument();

    finishExport({ csvPath: null, latexDirectory: "/tmp/Test-latex", zipPath: "/tmp/Test.zip", pdfPath: "/tmp/Test.pdf", pdfStatus: "created", rowCount: 1, issues: [], diagnosticPath: null });
    expect(await screen.findByText("Export complete")).toBeInTheDocument();
  });

  it("defers dependency checks until the requirements step and skips TeX for sources", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    expect(screen.getByLabelText("PDF")).toBeChecked();
    expect(backendMock.checkLatexEnvironment).not.toHaveBeenCalled();
    choose("LaTeX");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    expect(backendMock.checkLatexEnvironment).not.toHaveBeenCalled();
  });

  it("renders the Overleaf flow and official help in Taiwan Traditional Chinese", async () => {
    await i18n.changeLanguage("zh-TW");
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.exportProject.mockResolvedValue({ csvPath: null, latexDirectory: "/tmp/Test-latex", zipPath: "/tmp/Test.zip", pdfPath: null, pdfStatus: "notRequested", rowCount: 1, issues: [], diagnosticPath: null });
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("Overleaf"));
    fireEvent.click(screen.getByRole("button", { name: "下一步" }));
    fireEvent.click(screen.getByRole("button", { name: "檢查準備狀態" }));
    await screen.findByText("可匯出 1 列");
    fireEvent.click(screen.getByRole("button", { name: "下一步" }));
    fireEvent.click(screen.getByRole("button", { name: "選擇位置並匯出" }));
    await screen.findByText("接著在 Overleaf 操作");
    fireEvent.click(screen.getByRole("button", { name: "開啟 Overleaf" }));
    expect(backendMock.openOverleaf).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "官方 ZIP 匯入教學" }));
    expect(backendMock.openExportHelp).toHaveBeenCalledWith("upload");
    expect(backendMock.checkLatexEnvironment).not.toHaveBeenCalled();
  });

  it("persists the optional direct related-entry mode for LaTeX", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    fireEvent.change(screen.getByLabelText("Related entries"), { target: { value: "both" } });
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
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
    choose("LaTeX");

    const pronunciation = screen.getByLabelText("Pronunciation writing system");
    expect(within(pronunciation).queryByRole("option", { name: "Traditional Chinese" })).not.toBeInTheDocument();
    expect(within(pronunciation).getByRole("option", { name: "IPA" })).toBeInTheDocument();
    expect(screen.getByText("Shown beside the headword and cannot use the same writing system.")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Headword writing system"), { target: { value: "ws2" } });
    expect(screen.getByLabelText("Pronunciation writing system")).toHaveValue("");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
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
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    await confirm();
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
    choose("LaTeX");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
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
    choose("LaTeX");
    expect(screen.getByText("Charis SIL (fixed for IPA)")).toBeInTheDocument();
  });

  it("offers both Chiron families and explains their typeface styles", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn(async () => undefined)} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");

    const font = screen.getByLabelText("Portable font for Traditional Chinese");
    expect(within(font).getByRole("option", { name: "Chiron Sung HK" })).toBeInTheDocument();
    expect(within(font).getByRole("option", { name: "Chiron Hei HK" })).toBeInTheDocument();

    fireEvent.change(font, { target: { value: "chironSungHk" } });
    expect(screen.getByText("Ming/Song style for Traditional Chinese.")).toBeInTheDocument();
    fireEvent.change(font, { target: { value: "chironHeiHk" } });
    expect(screen.getByText("Hei/sans-serif style for Traditional Chinese.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await waitFor(() => expect(backendMock.saveExportSettings).toHaveBeenCalledWith(expect.objectContaining({
      latex: expect.objectContaining({ fontPresets: { ws1: "chironHeiHk" } }),
    })));
  });
  it("blocks PDF until installation is rechecked, without treating installer handoff as success", async () => {
    backendMock.checkLatexEnvironment.mockResolvedValue({ state: "missing", path: null, version: null, missingFiles: [], diagnosticPath: null });
    backendMock.installLatex.mockImplementation(async (onProgress) => onProgress({ phase: "waiting", downloadedBytes: 100, totalBytes: 100 }));
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("XeLaTeX is not installed. Install it here, or choose Overleaf.");
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Install LaTeX" }));
    await screen.findByText(/Official installer opened/);
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
    backendMock.checkLatexEnvironment.mockResolvedValue({ state: "ready", path: "/tex/xelatex", version: "XeTeX", missingFiles: [], diagnosticPath: null });
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Next" })).toBeEnabled());
  });

  it("preserves settings on Back and builds a fresh preview after edits", async () => {
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    fireEvent.change(screen.getByLabelText("Dictionary title"), { target: { value: "My dictionary" } });
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByLabelText("Dictionary title")).toHaveValue("My dictionary");
    fireEvent.change(screen.getByLabelText("Dictionary title"), { target: { value: "Updated" } });
    expect(screen.queryByText("1 rows ready")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await waitFor(() => expect(backendMock.previewExport).toHaveBeenCalledTimes(2));
    expect(backendMock.saveExportSettings).toHaveBeenLastCalledWith(expect.objectContaining({ latex: expect.objectContaining({ title: "Updated" }) }));
  });

  it("stays on confirmation when the destination is cancelled", async () => {
    backendMock.chooseFolder.mockResolvedValue(null);
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("LaTeX");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    await confirm();
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Choose destination and export" })).toBeEnabled());
    expect(backendMock.exportProject).not.toHaveBeenCalled();
  });

  it("shows a download failure and allows a safe retry", async () => {
    backendMock.checkLatexEnvironment.mockResolvedValue({ state: "missing", path: null, version: null, missingFiles: [], diagnosticPath: null });
    backendMock.installLatex.mockRejectedValueOnce(new CommandError("latex_install_integrity", "mismatch"));
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    fireEvent.click(await screen.findByRole("button", { name: "Install LaTeX" }));
    await screen.findByText(/Installer verification failed/);
    expect(screen.getByRole("button", { name: "Install LaTeX" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
  });

  it("keeps navigation locked during download and supports cancellation", async () => {
    backendMock.checkLatexEnvironment.mockResolvedValue({ state: "missing", path: null, version: null, missingFiles: [], diagnosticPath: null });
    let rejectInstall!: (value: unknown) => void;
    backendMock.installLatex.mockImplementation(() => new Promise((_, reject) => { rejectInstall = reject; }));
    backendMock.cancelLatexDownload.mockImplementation(async () => { rejectInstall(new CommandError("latex_install_cancelled", "cancelled")); });
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    fireEvent.click(await screen.findByRole("button", { name: "Install LaTeX" }));
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Close" })).toBeDisabled();
    const cancel = screen.getAllByRole("button", { name: "Cancel" }).find((button) => !(button as HTMLButtonElement).disabled)!;
    fireEvent.click(cancel);
    await screen.findByText("Installer download cancelled.");
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Install LaTeX" })).toBeEnabled();
  });

  it("does not announce PDF completion when the compiler disappears after checking", async () => {
    backendMock.chooseFolder.mockResolvedValue("/tmp");
    backendMock.exportProject.mockResolvedValue({ csvPath: null, latexDirectory: "/tmp/latex", zipPath: "/tmp/export.zip", pdfPath: null, pdfStatus: "xeLatexMissing", rowCount: 1, issues: [], diagnosticPath: null });
    render(<ExportDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} onSetAnalysisLanguage={vi.fn()} onNavigateEntry={vi.fn()} />);
    choose("PDF");
    fireEvent.click(screen.getByRole("button", { name: "Check requirements" }));
    await screen.findByText("1 rows ready");
    await confirm();
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and export" }));
    await screen.findByText("Sources saved; PDF was not created");
    expect(screen.queryByText("Export complete")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Official ZIP import guide" })).toBeInTheDocument();
  });

});
