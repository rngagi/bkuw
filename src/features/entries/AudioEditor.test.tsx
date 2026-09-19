import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { AudioAttachment, LexicalEntry } from "../../types/domain";

const backendMock = vi.hoisted(() => ({ listAudio: vi.fn(), chooseAudioFiles: vi.fn(), importAudio: vi.fn(), loadAudio: vi.fn(), removeAudio: vi.fn() }));
vi.mock("../../lib/tauri", () => ({ backend: backendMock, CommandError: class extends Error { constructor(public code: string, message: string) { super(message); } } }));
import { AudioEditor } from "./AudioEditor";
const entry: LexicalEntry = { id: "entry", revision: 1, notes: null, sectionOverride: null, createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z", forms: [], senses: [], relations: [] };
const clip: AudioAttachment = { id: "clip", owner: { kind: "example", id: "example" }, originalFilename: "語音.wav", durationMs: 2500, byteSize: 20000, sortOrder: 0, createdAt: "2026-01-01Z" };
const props = { entryId: entry.id, owner: clip.owner, onFlush: vi.fn(), onEntryMutated: vi.fn() };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }

describe("AudioEditor", () => {
  beforeEach(async () => {
    vi.restoreAllMocks();
    vi.resetAllMocks();
    await i18n.changeLanguage("en");
    backendMock.listAudio.mockResolvedValue([]);
    backendMock.chooseAudioFiles.mockResolvedValue([]);
    backendMock.loadAudio.mockResolvedValue({ mimeType: "audio/mpeg", dataBase64: "SUQz" });
    props.onFlush.mockResolvedValue(entry);
    let counter = 0;
    vi.stubGlobal("URL", { createObjectURL: vi.fn(() => `blob:test-${++counter}`), revokeObjectURL: vi.fn() });
    const states = new WeakMap<HTMLMediaElement, boolean>();
    vi.spyOn(HTMLMediaElement.prototype, "paused", "get").mockImplementation(function (this: HTMLMediaElement) { return !(states.get(this) ?? false); });
    vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(async function (this: HTMLMediaElement) { states.set(this, true); this.dispatchEvent(new Event("play")); });
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(function (this: HTMLMediaElement) { states.set(this, false); this.dispatchEvent(new Event("pause")); });
  });
  it("imports multiple files with a fresh revision, keeps successes after failure, and removes one", async () => {
    backendMock.chooseAudioFiles.mockResolvedValue(["/音檔/語音.wav", "/bad.wav", "/next.flac"]);
    backendMock.importAudio.mockResolvedValueOnce({ entry: { ...entry, revision: 2 }, audio: clip })
      .mockRejectedValueOnce(new Error("invalid"))
      .mockResolvedValueOnce({ entry: { ...entry, revision: 3 }, audio: { ...clip, id: "second", originalFilename: "next.flac" } });
    props.onFlush.mockResolvedValueOnce(entry).mockResolvedValueOnce({ ...entry, revision: 2 }).mockResolvedValueOnce({ ...entry, revision: 2 }).mockResolvedValue({ ...entry, revision: 3 });
    backendMock.removeAudio.mockResolvedValue({ entry: { ...entry, revision: 4 }, audio: null });
    render(<AudioEditor {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "Add audio" }));
    expect(await screen.findByText("next.flac")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("bad.wav");
    expect(backendMock.importAudio).toHaveBeenNthCalledWith(1, { entryId: "entry", owner: clip.owner, expectedRevision: 1, sourcePath: "/音檔/語音.wav" });
    expect(backendMock.importAudio.mock.calls[2][0].expectedRevision).toBe(2);
    expect(props.onEntryMutated).toHaveBeenCalledTimes(2);
    fireEvent.click(screen.getByRole("button", { name: "Remove 語音.wav" }));
    await waitFor(() => expect(screen.queryByText("語音.wav")).not.toBeInTheDocument());
    expect(backendMock.removeAudio).toHaveBeenCalledWith({ entryId: "entry", audioId: "clip", expectedRevision: 3 });
    expect(screen.getByText("next.flac")).toBeInTheDocument();
  });
  it("cancels file selection without saving and ignores results after leaving the owner", async () => {
    const pending = deferred<{ entry: LexicalEntry; audio: AudioAttachment }>();
    const rendered = render(<AudioEditor {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "Add audio" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Add audio" })).toBeEnabled());
    expect(props.onFlush).not.toHaveBeenCalled();
    backendMock.chooseAudioFiles.mockResolvedValue(["/voice.wav", "/later.wav"]);
    backendMock.importAudio.mockReturnValue(pending.promise);
    fireEvent.click(screen.getByRole("button", { name: "Add audio" }));
    await waitFor(() => expect(backendMock.importAudio).toHaveBeenCalledTimes(1));
    rendered.unmount();
    await act(async () => pending.resolve({ entry, audio: clip }));
    expect(props.onEntryMutated).not.toHaveBeenCalled();
    expect(backendMock.importAudio).toHaveBeenCalledTimes(1);
  });
  it("loads lazily, plays only one clip, seeks, and releases URLs when leaving", async () => {
    backendMock.listAudio.mockResolvedValue([clip, { ...clip, id: "second", originalFilename: "second.wav" }]);
    const { container, unmount } = render(<AudioEditor {...props} />);
    await screen.findByText("語音.wav");
    expect(backendMock.loadAudio).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Play 語音.wav" }));
    await screen.findByRole("button", { name: "Pause 語音.wav" });
    fireEvent.change(screen.getByRole("slider", { name: "Playback position for 語音.wav" }), { target: { value: "1.2" } });
    expect(container.querySelector("audio")?.currentTime).toBe(1.2);
    fireEvent.click(screen.getByRole("button", { name: "Play second.wav" }));
    await screen.findByRole("button", { name: "Pause second.wav" });
    expect(screen.getByRole("button", { name: "Play 語音.wav" })).toBeInTheDocument();
    unmount();
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(2);
  });
  it("does not start stale playback after another clip was requested", async () => {
    const pending = deferred<{ mimeType: string; dataBase64: string }>();
    backendMock.listAudio.mockResolvedValue([clip, { ...clip, id: "second", originalFilename: "second.wav" }]);
    backendMock.loadAudio.mockReturnValueOnce(pending.promise);
    render(<AudioEditor {...props} />);
    await screen.findByText("語音.wav");
    fireEvent.click(screen.getByRole("button", { name: "Play 語音.wav" }));
    fireEvent.click(screen.getByRole("button", { name: "Play second.wav" }));
    await screen.findByRole("button", { name: "Pause second.wav" });
    await act(async () => pending.resolve({ mimeType: "audio/mpeg", dataBase64: "SUQz" }));
    expect(screen.getByRole("button", { name: "Play 語音.wav" })).toBeInTheDocument();
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledTimes(1);
  });
  it("shows playback failure in Taiwan Traditional Chinese", async () => {
    await i18n.changeLanguage("zh-TW");
    backendMock.listAudio.mockResolvedValue([clip]);
    backendMock.loadAudio.mockRejectedValue(new Error("corrupt"));
    render(<AudioEditor {...props} />);
    await screen.findByText("語音.wav");
    fireEvent.click(screen.getByRole("button", { name: "播放 語音.wav" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("無法播放");
    expect(screen.getByRole("button", { name: "添加音檔" })).toBeEnabled();
  });
});
