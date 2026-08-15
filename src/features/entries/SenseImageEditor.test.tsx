import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { LexicalEntry, SenseImage } from "../../types/domain";

const { backendMock, prepareMock } = vi.hoisted(() => ({
  backendMock: {
    listSenseImages: vi.fn(),
    attachSenseImage: vi.fn(),
    loadSenseImage: vi.fn(),
    removeSenseImage: vi.fn(),
  },
  prepareMock: vi.fn(),
}));

vi.mock("../../lib/imageCompression", async (importOriginal) => ({
  ...await importOriginal<typeof import("../../lib/imageCompression")>(),
  prepareSenseImage: prepareMock,
}));
vi.mock("../../lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error {
    constructor(public code: string, message: string, public details?: string) { super(message); }
  },
}));

import { SenseImageEditor } from "./SenseImageEditor";

const entry: LexicalEntry = {
  id: "entry-1", notes: null, sectionOverride: null, revision: 1,
  createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z", forms: [], senses: [], relations: [],
};
const image: SenseImage = {
  id: "image-1", senseId: "sense-1", originalFilename: "field.jpg",
  width: 1600, height: 1200, byteSize: 420000, sortOrder: 0, createdAt: "2026-01-01Z",
};

describe("SenseImageEditor", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    backendMock.listSenseImages.mockResolvedValue([]);
    backendMock.loadSenseImage.mockResolvedValue({ mimeType: "image/png", dataBase64: "iVBORw0KGgo=" });
    prepareMock.mockResolvedValue({ originalFilename: "field.jpg", pngBase64: "iVBORw==", width: 1600, height: 1200 });
  });

  it("flushes before attaching and removing a lightly processed PNG", async () => {
    const updated = { ...entry, revision: 2 };
    const removed = { ...entry, revision: 3 };
    const onFlush = vi.fn()
      .mockResolvedValueOnce(entry)
      .mockResolvedValueOnce(updated);
    const onEntryMutated = vi.fn();
    backendMock.attachSenseImage.mockResolvedValue({ entry: updated, image });
    backendMock.removeSenseImage.mockResolvedValue({ entry: removed, image: null });
    const { container } = render(<SenseImageEditor entryId="entry-1" senseId="sense-1" onFlush={onFlush} onEntryMutated={onEntryMutated} />);

    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [new File(["jpeg"], "field.jpg", { type: "image/jpeg" })] } });
    await waitFor(() => expect(backendMock.attachSenseImage).toHaveBeenCalledWith({
      entryId: "entry-1", senseId: "sense-1", expectedRevision: 1,
      originalFilename: "field.jpg", pngBase64: "iVBORw==",
    }));
    expect(onEntryMutated).toHaveBeenCalledWith(updated);
    expect(await screen.findByText("field.jpg")).toBeInTheDocument();
    expect(await screen.findByRole("img", { name: "field.jpg" })).toHaveAttribute(
      "src",
      "data:image/png;base64,iVBORw0KGgo=",
    );

    fireEvent.click(screen.getByRole("button", { name: "Remove photo" }));
    await waitFor(() => expect(backendMock.removeSenseImage).toHaveBeenCalledWith({
      entryId: "entry-1", imageId: "image-1", expectedRevision: 2,
    }));
    expect(onEntryMutated).toHaveBeenLastCalledWith(removed);
  });

  it.each([
    ["a backend failure", "en", "Photo preview unavailable.", () => Promise.reject(new Error("read failed"))],
    ["an invalid payload", "zh-TW", "無法載入相片預覽。", () => Promise.resolve({ mimeType: "image/png" as const, dataBase64: "aGVsbG8=" })],
  ])("shows an explicit preview error for %s", async (_case, locale, message, response) => {
    await i18n.changeLanguage(locale);
    backendMock.listSenseImages.mockResolvedValue([image]);
    backendMock.loadSenseImage.mockImplementation(response);
    render(<SenseImageEditor entryId="entry-1" senseId="sense-1" onFlush={vi.fn()} onEntryMutated={vi.fn()} />);

    expect(await screen.findByText(message)).toBeInTheDocument();
    expect(screen.queryByText(locale === "en" ? "Loading…" : "載入中…")).not.toBeInTheDocument();
  });
});
