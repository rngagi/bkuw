import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { CsvImportWizard } from "./CsvImportWizard";

const backendMock = vi.hoisted(() => ({
  chooseCsvFile: vi.fn(), inspectCsv: vi.fn(), previewCsvImport: vi.fn(),
  chooseFolder: vi.fn(), createProjectFromCsv: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({ backend: backendMock }));

describe("CsvImportWizard", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    backendMock.chooseCsvFile.mockResolvedValue("/tmp/rngagi.csv");
    backendMock.inspectCsv.mockResolvedValue({
      sourcePath: "/tmp/rngagi.csv", fileName: "rngagi.csv", sha256: "abc", delimiter: "comma", rowCount: 2,
      profile: "rngagi-corpus-v0.3",
      columns: ["form", "gloss_zh", "word_root", "example", "example_translation_zh", "ipa", "part_of_speech", "gloss_en", "notes"].map((name, index) => ({ index, name, samples: [index === 0 ? "ama" : "sample"] })),
    });
    backendMock.previewCsvImport.mockResolvedValue({
      previewToken: "token", sourceRowCount: 2, importEntryCount: 1, importSenseCount: 2,
      skippedRowCount: 0, blockingErrorCount: 0, warningCount: 0, issues: [],
      groups: [{ rowIndices: [0, 1], primaryForm: "ama", blocked: false }],
    });
  });

  it("prefills the rngagi mapping and opens the grouping preview", async () => {
    render(<CsvImportWizard onCancel={vi.fn()} onProject={vi.fn()} onError={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose CSV file" }));
    await screen.findByText("rngagi-corpus v0.3 was detected and its nine columns were pre-mapped.");
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Project and writing systems" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect((screen.getByLabelText("Map form") as HTMLSelectElement).value).toMatch(/^entryForm:/);
    expect(screen.getByLabelText("Map notes")).toHaveValue("entryNotes");
    fireEvent.click(screen.getByRole("button", { name: "Preview import" }));
    await waitFor(() => expect(backendMock.previewCsvImport).toHaveBeenCalled());
    expect(await screen.findByText("Source rows 2, 3")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Split" })).toBeInTheDocument();
  });
});
