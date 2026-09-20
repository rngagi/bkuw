import { Mic, Pause, Play, RotateCcw, Square } from "lucide-react";
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { claimPlayback, isMicrophoneBusy, recordingBase64, releasePlayback, startCapture, subscribeMicrophone } from "../../lib/audioCapture";
import { backend, CommandError } from "../../lib/tauri";
import type { AudioMutation, AudioOwner, LexicalEntry } from "../../types/domain";

interface Props {
  entryId: string;
  owner: AudioOwner;
  disabled: boolean;
  onFlush(): Promise<LexicalEntry | undefined>;
  onSaved(result: AudioMutation): void;
}
export function AudioRecorder({ entryId, owner, disabled, onFlush, onSaved }: Props) {
  const { t } = useTranslation();
  const microphoneBusy = useSyncExternalStore(subscribeMicrophone, isMicrophoneBusy);
  const [phase, setPhase] = useState<"idle" | "requesting" | "recording" | "stopping" | "review" | "saving">("idle");
  const [elapsed, setElapsed] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const player = useRef<HTMLAudioElement>(null);
  const url = useRef<string | null>(null);
  const blob = useRef<Blob | null>(null);
  const token = useRef("");
  const capture = useRef<ReturnType<typeof startCapture> | null>(null);
  const generation = useRef(0);
  const running = useRef(false);
  function clearPreview() {
    setPlaying(false);
    releasePlayback(player.current);
    player.current?.removeAttribute("src");
    if (url.current) URL.revokeObjectURL(url.current);
    url.current = null; blob.current = null;
  }
  useEffect(() => {
    const element = player.current;
    return () => {
      generation.current++; capture.current?.cancel(); releasePlayback(element);
      if (url.current) URL.revokeObjectURL(url.current);
      url.current = null; blob.current = null;
    };
  }, []);
  function discard() {
    generation.current++; capture.current?.cancel(); capture.current = null;
    running.current = false; clearPreview(); setPhase("idle"); setElapsed(0);
  }
  async function begin() {
    if (running.current || disabled || isMicrophoneBusy()) return;
    running.current = true; clearPreview(); setError(null); setElapsed(0); setPhase("requesting");
    const current = ++generation.current;
    try {
      const entry = await onFlush();
      if (generation.current !== current) return;
      if (!entry || entry.id !== entryId) throw new Error("audio_stale");
      token.current = await backend.beginAudioRecording({ entryId, owner, expectedRevision: entry.revision });
      if (generation.current !== current) return;
      capture.current = startCapture({
        onStarted: () => { if (generation.current === current) setPhase("recording"); },
        onElapsed: (seconds) => { if (generation.current === current) setElapsed(seconds); },
        onComplete: (value) => {
          if (generation.current !== current) return;
          blob.current = value; url.current = URL.createObjectURL(value);
          if (player.current) player.current.src = url.current;
          running.current = false; setPhase("review");
        },
        onError: (key) => { if (generation.current === current) { running.current = false; setError(key); setPhase("idle"); } },
      });
    } catch (cause) {
      if (generation.current === current) { running.current = false; setError(errorKey(cause)); setPhase("idle"); }
    }
  }
  async function save() {
    if (running.current || !blob.current) return;
    running.current = true; setPhase("saving"); setError(null); releasePlayback(player.current);
    const current = generation.current;
    const recording = blob.current;
    try {
      const dataBase64 = await recordingBase64(recording);
      if (generation.current !== current) return;
      const entry = await onFlush();
      if (generation.current !== current) return;
      if (!entry || entry.id !== entryId) throw new Error("audio_stale");
      const result = await backend.saveAudioRecording({ entryId, owner, expectedRevision: entry.revision, sessionToken: token.current, mimeType: recording.type, originalFilename: t("audio.recordName", { date: new Date().toLocaleString() }), dataBase64 });
      if (generation.current !== current) return;
      onSaved(result); discard();
    } catch (cause) {
      if (generation.current === current) { running.current = false; setError(errorKey(cause)); setPhase("review"); }
    }
  }
  return <div className="audio-recorder">
    <audio ref={player} preload="none" onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => setPlaying(false)} onError={() => { setPlaying(false); setError("audio.playFailed"); }} />
    {phase === "idle" && <Button type="button" size="small" disabled={disabled || microphoneBusy} onClick={() => void begin()}><Mic size={14} />{t("audio.record")}</Button>}
    {(phase === "requesting" || phase === "stopping" || phase === "saving") && <span role="status">{t(`audio.${phase}`)}</span>}
    {phase === "recording" && <><span role="status">{t("audio.recording", { time: `${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}` })}</span><Button type="button" size="small" onClick={() => { setPhase("stopping"); capture.current?.stop(); }}><Square size={14} />{t("audio.stop")}</Button></>}
    {phase === "review" && <>
      <Button type="button" size="small" disabled={microphoneBusy} onClick={() => { const element = player.current; if (!element) return; if (playing) { element.pause(); return; } if (claimPlayback(element) !== null) void element.play().catch(() => setError("audio.playFailed")); }}>{playing ? <Pause size={14} /> : <Play size={14} />}{t(playing ? "audio.pausePreview" : "audio.preview")}</Button>
      <Button type="button" size="small" disabled={disabled} onClick={() => void save()}>{t("audio.saveRecording")}</Button>
      <Button type="button" size="small" disabled={disabled || microphoneBusy} onClick={() => void begin()}><RotateCcw size={14} />{t("audio.rerecord")}</Button>
    </>}
    {phase !== "idle" && phase !== "saving" && <Button type="button" size="small" variant="ghost" onClick={discard}>{t("common.cancel")}</Button>}
    {error && <p role="alert" className="image-error">{t(error, { defaultValue: t("audio.recordFailed") })}</p>}
  </div>;
}
function errorKey(cause: unknown) {
  if (cause instanceof CommandError) return `error.${cause.code}`;
  return cause instanceof Error && cause.message === "audio_stale" ? "error.audio_stale" : "audio.recordFailed";
}
