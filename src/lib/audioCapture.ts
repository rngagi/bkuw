let microphoneBusy = false;
const listeners = new Set<() => void>();
let player: HTMLAudioElement | null = null;
let playback = 0;
export const isMicrophoneBusy = () => microphoneBusy;
export function subscribeMicrophone(listener: () => void) {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
function setBusy(value: boolean) { microphoneBusy = value; listeners.forEach((listener) => listener()); }
export function claimPlayback(element: HTMLAudioElement) {
  if (microphoneBusy) return null;
  player?.pause(); player = element;
  return ++playback;
}
export const isCurrentPlayback = (request: number) => request === playback && !microphoneBusy;
export function releasePlayback(element: HTMLAudioElement | null) {
  element?.pause();
  if (player === element) { player = null; playback++; }
}

export interface CaptureEvents {
  onStarted(): void;
  onElapsed(seconds: number): void;
  onComplete(blob: Blob): void;
  onError(key: string): void;
}
export function startCapture(events: CaptureEvents) {
  let canceled = false;
  let finished = false;
  let stream: MediaStream | undefined;
  let recorder: MediaRecorder | undefined;
  let timer: ReturnType<typeof setInterval> | undefined;
  const chunks: Blob[] = [];
  let size = 0;
  const ownsMicrophone = !microphoneBusy;
  function release() {
    clearInterval(timer);
    stream?.getTracks().forEach((track) => { track.onended = null; track.stop(); });
    if (ownsMicrophone) setBusy(false);
  }
  function fail(key: string) {
    if (finished || canceled) return;
    finished = true;
    try { if (recorder?.state === "recording") recorder.stop(); } catch { /* Release a failed device below. */ }
    release(); events.onError(key);
  }
  function stop() {
    try { if (!finished && !canceled && recorder?.state === "recording") recorder.stop(); }
    catch { fail("audio.recordFailed"); }
  }
  if (ownsMicrophone) { setBusy(true); releasePlayback(player); }
  void (async () => {
    try {
      if (!ownsMicrophone) { fail("audio.recordBusy"); return; }
      if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") { fail("audio.recordUnsupported"); return; }
      const mimeType = ["audio/webm;codecs=opus", "audio/mp4", "audio/webm"].find((type) => MediaRecorder.isTypeSupported(type));
      if (!mimeType) { fail("audio.recordUnsupported"); return; }
      stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1, echoCancellation: false, noiseSuppression: false, autoGainControl: false }, video: false });
      if (canceled) { stream.getTracks().forEach((track) => track.stop()); return; }
      recorder = new MediaRecorder(stream, { mimeType, audioBitsPerSecond: 128000 });
      recorder.ondataavailable = (event) => {
        if (canceled || finished) return;
        size += event.data.size;
        if (size > 64 * 1024 * 1024) { fail("audio.recordLimit"); return; }
        if (event.data.size) chunks.push(event.data);
      };
      recorder.onerror = () => fail("audio.recordFailed");
      recorder.onstop = () => {
        if (canceled || finished) return;
        finished = true; release();
        const blob = new Blob(chunks, { type: recorder!.mimeType });
        if (blob.size) events.onComplete(blob); else events.onError("audio.recordEmpty");
      };
      stream.getTracks().forEach((track) => { track.onended = () => fail("audio.recordFailed"); });
      recorder.start(1000);
      const started = performance.now();
      timer = setInterval(() => {
        const seconds = Math.floor((performance.now() - started) / 1000);
        events.onElapsed(Math.min(seconds, 1800));
        // Leave room for a final encoder chunk before the backend's 30-minute limit.
        if (seconds >= 1799) stop();
      }, 250);
      events.onStarted();
    } catch (error) {
      fail(error instanceof DOMException && ["NotAllowedError", "SecurityError"].includes(error.name) ? "audio.recordDenied" : "audio.recordFailed");
    }
  })();
  return {
    stop,
    cancel() {
      if (finished || canceled) return;
      canceled = true;
      try { if (recorder?.state === "recording") recorder.stop(); } catch { /* Cancellation still releases the device. */ } finally { release(); }
    },
  };
}

export function recordingBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1]);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}
