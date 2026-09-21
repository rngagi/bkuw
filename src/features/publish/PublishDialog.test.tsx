import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { CommandError } from "../../lib/tauri";
import type { ProjectSnapshot, PublishSettings, PublishState } from "../../types/domain";

const { backendMock } = vi.hoisted(() => ({
  backendMock: {
    getPublishState: vi.fn(), openPublishHelp: vi.fn(), openCloudflareTokenPage: vi.fn(),
    connectCloudflare: vi.fn(), disconnectCloudflare: vi.fn(), savePublishSettings: vi.fn(),
    previewPublish: vi.fn(), publishSite: vi.fn(), cancelPublish: vi.fn(),
    openPublicWebsite: vi.fn(), retryPublishCleanup: vi.fn(),
  },
}));
vi.mock("../../lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error {
    constructor(public code: string, message: string, public details?: string) { super(message); }
  },
}));

import { PublishDialog } from "./PublishDialog";

const snapshot: ProjectSnapshot = {
  rootPath: "/tmp/Test.bkuw",
  project: { id: "p1", name: "Test", languageName: "Test language", languageCode: "tst", analysisLanguage: "en", description: null, createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z" },
  writingSystems: [
    { id: "primary", name: "Orthography", type: "orthography", scriptCode: "Latn", languageTag: "en", displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null },
    { id: "ipa", name: "IPA", type: "phonetic", scriptCode: "Latn", languageTag: null, displayRole: null, sortOrder: 1, fontFamily: null, notes: null },
  ],
  partOfSpeechOptions: [], semanticDomainOptions: [], entries: [],
  exportSettings: { version: 1, corpus: { partOfSpeechMappings: {} }, latex: { title: "Test", author: "", headwordWritingSystemId: "primary", pronunciationWritingSystemId: "ipa", exampleWritingSystemId: "primary", collationLanguageTag: null, sectionMode: "auto", reverseIndex: "none", relatedEntries: "none", includeSenseImages: false, includeSemanticDomains: true, fontPresets: {} } },
  entrySortSettings: { version: 2, mode: "auto", source: "writingSystem", writingSystemId: "primary", alphabet: [] },
  manualSortLayout: { version: 1, items: [] },
};
const settings: PublishSettings = { version: 1, title: "Test dictionary", description: null, locale: "en", infoMarkdown: null, includeEntryNotes: false, includeExampleNotes: false, includeRelations: false, writingSystemIds: ["primary", "ipa"], workerName: "bkuw-test-12345678", bucketName: "bkuw-test-12345678-media" };
const connected: PublishState = { settings, deployment: null, connection: { connected: true, accountId: "0123456789abcdef0123456789abcdef", workersSubdomain: "test-dictionary", credentialPersisted: true } };

describe("PublishDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en"); vi.resetAllMocks();
    backendMock.getPublishState.mockResolvedValue(connected);
    backendMock.savePublishSettings.mockImplementation(async (value) => value);
    backendMock.previewPublish.mockResolvedValue({ snapshotToken: "snapshot", publicUrl: "https://bkuw-test-12345678.test-dictionary.workers.dev", entryCount: 2, senseCount: 3, exampleCount: 1, imageCount: 1, audioCount: 2, uploadMediaCount: 3, unchangedMediaCount: 0, deleteMediaCount: 0, uploadBytes: 1024, issues: [] });
    backendMock.publishSite.mockImplementation(async (_token, onProgress) => {
      onProgress({ phase: "complete", completedItems: 1, totalItems: 1, uploadedBytes: 1024, totalBytes: 1024 });
      return { publicUrl: "https://bkuw-test-12345678.test-dictionary.workers.dev", uploadedMediaCount: 3, unchangedMediaCount: 0, deletedMediaCount: 0, cleanupPending: false, lastPublishedAt: "2026-09-21T00:00:00Z" };
    });
  });

  async function reachAddress() {
    render(<PublishDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} />);
    await screen.findByText("Your dictionary website will be public to anyone who knows its URL.");
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    await screen.findByLabelText(/^Worker name/);
  }

  async function reachSettings() {
    await reachAddress();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    await screen.findByLabelText("Website title");
  }

  it("keeps the primary writing system public and saves optional public fields before preview", async () => {
    const flush = vi.fn(async () => undefined);
    render(<PublishDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={flush} />);
    await screen.findByText("Your dictionary website will be public to anyone who knows its URL.");
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(await screen.findByRole("button", { name: "Next" }));
    expect(screen.getByLabelText(/Orthography/)).toBeDisabled();
    fireEvent.click(screen.getByLabelText("Show entry notes")); fireEvent.click(screen.getByLabelText("IPA"));
    fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await waitFor(() => expect(flush).toHaveBeenCalled());
    expect(backendMock.savePublishSettings).toHaveBeenCalledWith(expect.objectContaining({ includeEntryNotes: true, writingSystemIds: ["primary"] }));
    expect(await screen.findByText("2 entries")).toBeInTheDocument();
  });

  it("publishes the checked immutable snapshot and shows the public URL", async () => {
    await reachSettings(); fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await screen.findByText("2 entries"); fireEvent.click(screen.getByRole("button", { name: "Publish" }));
    await waitFor(() => expect(backendMock.publishSite).toHaveBeenCalledWith("snapshot", expect.any(Function)));
    expect(await screen.findByText("Website published")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "https://bkuw-test-12345678.test-dictionary.workers.dev" })).toBeInTheDocument();
  });

  it("requires the novice account checklist before continuing", async () => {
    backendMock.getPublishState.mockResolvedValue({ ...connected, connection: { connected: false, accountId: null, workersSubdomain: null, credentialPersisted: false } });
    render(<PublishDialog open snapshot={snapshot} onOpenChange={vi.fn()} onFlush={vi.fn()} />);
    const next = await screen.findByRole("button", { name: "Next" }); expect(next).toBeDisabled();
    fireEvent.click(screen.getByLabelText("I verified my email and can sign in")); fireEvent.click(screen.getByLabelText("I completed the R2 setup"));
    expect(next).toBeEnabled(); fireEvent.click(next); expect(await screen.findByLabelText("Account ID")).toBeInTheDocument();
  });

  it("explains the URL parts and derives the bucket from the Worker name", async () => {
    await reachAddress();
    expect(screen.getByText("The Worker name identifies this dictionary. The account subdomain is shared by every Worker in the same Cloudflare account.")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/^Worker name/), { target: { value: "renamed-dictionary" } });
    expect(screen.getByText("renamed-dictionary-media")).toBeInTheDocument();
    expect(screen.getByText("https://renamed-dictionary.test-dictionary.workers.dev")).toBeInTheDocument();
  });

  it("shows safe Cloudflare error details", async () => {
    backendMock.publishSite.mockRejectedValue(new CommandError("cloudflare_api", "Cloudflare failed", "10021: Missing main module filename"));
    await reachSettings(); fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await screen.findByText("2 entries"); fireEvent.click(screen.getByRole("button", { name: "Publish" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("10021: Missing main module filename");
  });
});
