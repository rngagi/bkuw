import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import type { LexicalEntry } from "../../types/domain";
import { isMicrophoneBusy, startCapture } from "../../lib/audioCapture";
const backendMock = vi.hoisted(() => ({ beginAudioRecording: vi.fn(), saveAudioRecording: vi.fn() }));
vi.mock("../../lib/tauri", () => ({ backend: backendMock, CommandError: class extends Error { constructor(public code: string) { super(code); } } }));
import { AudioRecorder } from "./AudioRecorder";
const entry: LexicalEntry = { id: "entry", revision: 1, notes: null, sectionOverride: null, createdAt: "2026-01-01Z", updatedAt: "2026-01-01Z", forms: [], senses: [], relations: [] };
let track: { stop: ReturnType<typeof vi.fn>; onended: (() => void) | null };
let getUserMedia: ReturnType<typeof vi.fn>;
class Recorder {
  static instances: Recorder[] = [];
  static isTypeSupported(type: string) { return type.startsWith("audio/webm"); }
  state = "inactive";
  mimeType = "audio/webm;codecs=opus";
  ondataavailable?: (event: { data: Blob }) => void;
  onstop?: () => void;
  onerror?: () => void;
  constructor() { Recorder.instances.push(this); }
  start() { this.state = "recording"; }
  stop() {
    this.state = "inactive";
    queueMicrotask(() => {
      this.ondataavailable?.({ data: new Blob(["recorded audio"], { type: this.mimeType }) });
      this.onstop?.();
    });
  }
}
const props = { entryId: entry.id, owner: { kind: "sense" as const, id: "sense" }, disabled: false, onFlush: vi.fn(), onSaved: vi.fn() };
async function startAndStop() {
  fireEvent.click(screen.getByRole("button", { name: "Record" }));
  fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  await screen.findByRole("button", { name: "Save recording" });
}
describe("AudioRecorder", () => {
  beforeEach(async () => {
    vi.restoreAllMocks(); vi.resetAllMocks();
    await i18n.changeLanguage("en");
    Recorder.instances = [];
    track = { stop: vi.fn(), onended: null };
    getUserMedia = vi.fn().mockResolvedValue({ getTracks: () => [track] });
    vi.stubGlobal("MediaRecorder", Recorder);
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: { getUserMedia } });
    vi.stubGlobal("URL", { createObjectURL: vi.fn(() => "blob:recording"), revokeObjectURL: vi.fn() });
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue();
    props.onFlush.mockResolvedValue(entry);
    backendMock.beginAudioRecording.mockResolvedValue("session");
    backendMock.saveAudioRecording.mockResolvedValue({ entry: { ...entry, revision: 2 }, audio: null });
  });
  afterEach(() => { vi.useRealTimers(); });
  it("records, previews, and saves only after explicit save with the latest revision", async () => {
    render(<AudioRecorder {...props} />);
    await startAndStop();
    expect(track.stop).toHaveBeenCalledTimes(1);
    expect(isMicrophoneBusy()).toBe(false);
    expect(backendMock.saveAudioRecording).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Preview recording" }));
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalled();
    props.onFlush.mockResolvedValue({ ...entry, revision: 3 });
    fireEvent.click(screen.getByRole("button", { name: "Save recording" }));
    await waitFor(() => expect(props.onSaved).toHaveBeenCalled());
    expect(backendMock.saveAudioRecording).toHaveBeenCalledWith(expect.objectContaining({ expectedRevision: 3, sessionToken: "session", owner: props.owner, mimeType: "audio/webm;codecs=opus", dataBase64: btoa("recorded audio") }));
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:recording");
  });
  it("keeps the preview after a failed save and supports retry and re-recording", async () => {
    render(<AudioRecorder {...props} />);
    await startAndStop();
    backendMock.saveAudioRecording.mockRejectedValueOnce(new Error("save failed"));
    fireEvent.click(screen.getByRole("button", { name: "Save recording" }));
    await screen.findByRole("alert");
    expect(screen.getByRole("button", { name: "Preview recording" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Record again" }));
    await screen.findByRole("button", { name: "Stop recording" });
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:recording");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(isMicrophoneBusy()).toBe(false);
    expect(props.onSaved).not.toHaveBeenCalled();
  });
  it("releases a late microphone grant after cancel and never starts a recorder", async () => {
    let grant!: (stream: unknown) => void;
    getUserMedia.mockReturnValue(new Promise((resolve) => { grant = resolve; }));
    render(<AudioRecorder {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "Record" }));
    await waitFor(() => expect(getUserMedia).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await act(async () => grant({ getTracks: () => [track] }));
    expect(track.stop).toHaveBeenCalledTimes(1);
    expect(Recorder.instances).toHaveLength(0);
    expect(isMicrophoneBusy()).toBe(false);
  });
  it("stops capturing on unmount and ignores an in-flight saved response", async () => {
    const view = render(<AudioRecorder {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "Record" }));
    await screen.findByRole("button", { name: "Stop recording" });
    view.unmount();
    expect(track.stop).toHaveBeenCalledTimes(1);
    expect(isMicrophoneBusy()).toBe(false);
    const second = render(<AudioRecorder {...props} />);
    await startAndStop();
    let complete!: (result: unknown) => void;
    backendMock.saveAudioRecording.mockReturnValue(new Promise((resolve) => { complete = resolve; }));
    fireEvent.click(screen.getByRole("button", { name: "Save recording" }));
    await waitFor(() => expect(backendMock.saveAudioRecording).toHaveBeenCalled());
    second.unmount();
    await act(async () => complete({ entry, audio: null }));
    expect(props.onSaved).not.toHaveBeenCalled();
  });
  it("reports denied permission in Traditional Chinese without saving", async () => {
    await i18n.changeLanguage("zh-TW");
    getUserMedia.mockRejectedValue(new DOMException("denied", "NotAllowedError"));
    render(<AudioRecorder {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "錄音" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("麥克風存取遭拒");
    expect(isMicrophoneBusy()).toBe(false);
    expect(backendMock.saveAudioRecording).not.toHaveBeenCalled();
  });
  it("allows only one microphone and stops automatically before the duration limit", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "performance"] });
    const events = { onStarted: vi.fn(), onElapsed: vi.fn(), onComplete: vi.fn(), onError: vi.fn() };
    const first = startCapture(events);
    await Promise.resolve();
    const competing = { ...events, onError: vi.fn() };
    startCapture(competing);
    expect(competing.onError).toHaveBeenCalledWith("audio.recordBusy");
    expect(getUserMedia).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1799 * 1000);
    expect(events.onComplete).toHaveBeenCalledTimes(1);
    expect(track.stop).toHaveBeenCalledTimes(1);
    expect(isMicrophoneBusy()).toBe(false);
    first.cancel();
  });
  it("rejects oversized captured data and releases the microphone", async () => {
    const events = { onStarted: vi.fn(), onElapsed: vi.fn(), onComplete: vi.fn(), onError: vi.fn() };
    startCapture(events);
    await Promise.resolve();
    Recorder.instances[0].ondataavailable?.({ data: { size: 64 * 1024 * 1024 + 1 } as Blob });
    await Promise.resolve();
    expect(events.onError).toHaveBeenCalledWith("audio.recordLimit");
    expect(events.onComplete).not.toHaveBeenCalled();
    expect(track.stop).toHaveBeenCalledTimes(1);
    expect(isMicrophoneBusy()).toBe(false);
  });
});
