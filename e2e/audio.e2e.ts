import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { $, browser, expect } from "@wdio/globals";

describe("bkuw offline audio", () => {
  it("imports with bundled tools and plays/seeks the verified WebM under the real WebView CSP", async () => {
    const parentDir = mkdtempSync(join(tmpdir(), "bkuw-audio-e2e-"));
    let opened = false;
    try {
      await browser.tauri.execute(({ core }, request) => core.invoke("create_project", { request }), { parentDir, name: "Audio smoke", languageName: null, languageCode: null });
      opened = true;
      let entry = await browser.tauri.execute(({ core }) => core.invoke("create_entry")) as any;
      const senseId = randomUUID(), exampleId = randomUUID();
      entry.senses = [{ id: senseId, gloss: "voice", definition: null, partOfSpeech: null, semanticDomain: null, sortOrder: 0, examples: [{ id: exampleId, translation: "example", notes: null, sortOrder: 0, forms: [] }] }];
      entry = await browser.tauri.execute(({ core }, request) => core.invoke("save_entry", { request }), { entry, expectedRevision: entry.revision });
      const support = await browser.execute(() => ({
        microphone: !!navigator.mediaDevices?.getUserMedia,
        recorder: typeof MediaRecorder !== "undefined",
        playback: document.createElement("audio").canPlayType('audio/webm;codecs=opus'),
      }));
      expect(support.microphone).toBe(true);
      expect(support.recorder).toBe(true);
      expect(support.playback).not.toBe("");
      const token = await browser.tauri.execute(({ core }, request) => core.invoke("begin_audio_recording", { request }), { entryId: entry.id, owner: { kind: "sense", id: senseId }, expectedRevision: entry.revision, sourcePath: "" });
      // Synthetic audio exercises the native MediaRecorder without accessing the user's microphone.
      const recorded = await browser.executeAsync((done) => {
        const context = new AudioContext();
        const source = context.createOscillator();
        const destination = context.createMediaStreamDestination();
        source.connect(destination); source.start();
        const mimeType = ["audio/webm;codecs=opus", "audio/mp4"].find((type) => MediaRecorder.isTypeSupported(type))!;
        const recorder = new MediaRecorder(destination.stream, { mimeType });
        const chunks: Blob[] = [];
        recorder.ondataavailable = (event) => chunks.push(event.data);
        recorder.onstop = () => {
          source.stop(); destination.stream.getTracks().forEach((track) => track.stop());
          void context.close();
          const blob = new Blob(chunks, { type: recorder.mimeType });
          const reader = new FileReader();
          reader.onload = () => done({ mimeType: blob.type, dataBase64: String(reader.result).split(",")[1] });
          reader.readAsDataURL(blob);
        };
        void context.resume().then(() => { recorder.start(100); setTimeout(() => recorder.stop(), 400); });
      }) as any;
      const saved = await browser.tauri.execute(({ core }, request) => core.invoke("save_audio_recording", { request }), { entryId: entry.id, owner: { kind: "sense", id: senseId }, expectedRevision: entry.revision, sessionToken: token, originalFilename: "Synthetic recording", ...recorded }) as any;
      expect(saved.audio.durationMs).toBeGreaterThan(100);
      entry = saved.entry;
      const senseAudio = await browser.tauri.execute(({ core }, request) => core.invoke("import_audio", { request }), { entryId: entry.id, owner: { kind: "sense", id: senseId }, expectedRevision: entry.revision, sourcePath: resolve("src-tauri/tests/fixtures/audio/tone.wav") }) as any;
      const exampleAudio = await browser.tauri.execute(({ core }, request) => core.invoke("import_audio", { request }), { entryId: entry.id, owner: { kind: "example", id: exampleId }, expectedRevision: senseAudio.entry.revision, sourcePath: resolve("src-tauri/tests/fixtures/audio/tone.opus") }) as any;
      expect(exampleAudio.audio.owner.kind).toBe("example");
      const content = await browser.tauri.execute(({ core }, audioId) => core.invoke("load_audio", { audioId }), saved.audio.id) as any;
      expect(content.mimeType).toBe("audio/webm");
      // A small native-media harness isolates platform decoder/CSP behavior from React unit tests.
      await browser.execute((payload) => {
        const audio = document.createElement("audio");
        audio.id = "audio-smoke-player";
        audio.loop = true;
        audio.src = URL.createObjectURL(new Blob([Uint8Array.from(atob(payload.dataBase64), (c) => c.charCodeAt(0))], { type: payload.mimeType }));
        document.body.append(audio);
        const button = document.createElement("button");
        button.id = "audio-smoke-play";
        button.textContent = "Play test audio";
        button.style.cssText = "position:fixed;top:0;left:0;z-index:2147483647";
        button.onclick = () => { void audio.play(); };
        document.body.append(button);
      }, content);
      await $("#audio-smoke-play").click();
      await browser.waitUntil(() => browser.execute(() => {
        const audio = document.querySelector<HTMLAudioElement>("#audio-smoke-player")!;
        return !audio.paused && audio.currentTime > 0 && audio.readyState >= 2;
      }), { timeout: 10_000, timeoutMsg: "WebM playback did not advance" });
      const seeked = await browser.executeAsync((done) => {
        const audio = document.querySelector<HTMLAudioElement>("#audio-smoke-player")!;
        audio.pause();
        audio.addEventListener("seeked", () => done(audio.currentTime), { once: true });
        audio.currentTime = 0.1;
      });
      expect(Number(seeked)).toBeGreaterThanOrEqual(0.08);
      await browser.tauri.execute(({ core }, request) => core.invoke("remove_audio", { request }), { entryId: entry.id, audioId: exampleAudio.audio.id, expectedRevision: exampleAudio.entry.revision });
      const remaining = await browser.tauri.execute(({ core }, owner) => core.invoke("list_audio", { owner }), { kind: "example", id: exampleId });
      expect(remaining).toEqual([]);
    } finally {
      await browser.execute(() => {
        const audio = document.querySelector<HTMLAudioElement>("#audio-smoke-player");
        if (audio) { audio.pause(); URL.revokeObjectURL(audio.src); audio.remove(); }
        document.querySelector("#audio-smoke-play")?.remove();
      });
      if (opened) await browser.tauri.execute(({ core }) => core.invoke("close_project"));
      rmSync(parentDir, { recursive: true, force: true });
    }
  });
});
