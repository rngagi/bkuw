import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { CsvImportWizard } from "./CsvImportWizard";

const backendMock = vi.hoisted(() => ({
  chooseCsvFile: vi.fn(), inspectCsv: vi.fn(), previewCsvImport: vi.fn(),
  chooseFolder: vi.fn(), createProjectFromCsv: vi.fn(),
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

  it("shows the exact CSV row-shape error in the wizard", async () => {
    const onError = vi.fn();
    backendMock.inspectCsv.mockRejectedValue(new CommandError(
      "csv_row_length",
      "A CSV row has a different number of columns than the header.",
      JSON.stringify({ row: 4, expected: 9, actual: 8 }),
    ));
    render(<CsvImportWizard onCancel={vi.fn()} onProject={vi.fn()} onError={onError} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose CSV file" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("CSV row 4 has 8 columns, but the header has 9. Check the selected delimiter and unmatched quotes.");
    expect(onError).not.toHaveBeenCalled();
  });

  it("localizes exact CSV parse details in Taiwan Traditional Chinese", async () => {
    await i18n.changeLanguage("zh-TW");
    backendMock.inspectCsv.mockRejectedValue(new CommandError(
      "csv_row_length",
      "A CSV row has a different number of columns than the header.",
      JSON.stringify({ row: 4, expected: 9, actual: 8 }),
    ));
    render(<CsvImportWizard onCancel={vi.fn()} onProject={vi.fn()} onError={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "選擇 CSV 檔" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("CSV 第 4 列有 8 欄，但標題列有 9 欄；請檢查分隔符與未成對的引號。");
  });

  it("names the conflicting source column and rows in preview issues", async () => {
    backendMock.previewCsvImport.mockResolvedValue({
      previewToken: "token", sourceRowCount: 2, importEntryCount: 1, importSenseCount: 2,
      skippedRowCount: 0, blockingErrorCount: 1, warningCount: 0,
      issues: [{ severity: "error", code: "group_entry_notes_conflict", rowIndices: [0, 1], columnIndices: [8], details: null }],
      groups: [{ rowIndices: [0, 1], primaryForm: "ama", blocked: true }],
    });
    render(<CsvImportWizard onCancel={vi.fn()} onProject={vi.fn()} onError={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose CSV file" }));
    await screen.findByText("rngagi-corpus v0.3 was detected and its nine columns were pre-mapped.");
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Preview import" }));
    expect(await screen.findByText(/Grouped rows have different entry notes in column “notes”.*rows 2, 3/)).toBeInTheDocument();
  });
});
