import { Music2, Pause, Play, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type { AudioAttachment, AudioOwner, LexicalEntry } from "../../types/domain";

interface Props {
  entryId: string;
  owner: AudioOwner;
  onFlush(): Promise<LexicalEntry | undefined>;
  onEntryMutated(entry: LexicalEntry): void;
}

export function AudioEditor({ entryId, owner, onFlush, onEntryMutated }: Props) {
  const { t } = useTranslation();
  const [items, setItems] = useState<AudioAttachment[]>([]);
  const [busy, setBusy] = useState(false);
  const [errors, setErrors] = useState<{ name: string; key: string }[]>([]);
  const [progress, setProgress] = useState<{ current: number; total: number } | null>(null);
  const generation = useRef(0);
  const running = useRef(false);
  useEffect(() => {
    const current = ++generation.current;
    setItems([]);
    void backend.listAudio({ kind: owner.kind, id: owner.id }).then((value) => {
      if (generation.current === current) setItems(value);
    }).catch((error: unknown) => {
      // A newly added owner is persisted by onFlush before its first import.
      if (generation.current === current && !(error instanceof CommandError && error.code === "audio_not_found")) {
        setErrors([{ name: "", key: errorKey(error) }]);
      }
    });
    return () => { generation.current++; };
  }, [owner.kind, owner.id]);

  async function add() {
    if (running.current) return;
    running.current = true;
    setBusy(true);
    setErrors([]);
    const current = generation.current;
    try {
      const paths = await backend.chooseAudioFiles();
      for (const [index, sourcePath] of paths.entries()) {
        if (current !== generation.current) break;
        setProgress({ current: index + 1, total: paths.length });
        try {
          const entry = await onFlush();
          if (current !== generation.current) break;
          if (!entry || entry.id !== entryId) throw new Error("audio_stale");
          const result = await backend.importAudio({ entryId, owner, expectedRevision: entry.revision, sourcePath });
          if (current !== generation.current) break;
          onEntryMutated(result.entry);
          if (result.audio) setItems((value) => [...value, result.audio!]);
        } catch (error) {
          if (current !== generation.current) break;
          setErrors((value) => [...value, { name: sourcePath.split(/[\\/]/).pop() ?? "", key: errorKey(error) }]);
        }
      }
    } catch (error) {
      if (current === generation.current) setErrors([{ name: "", key: errorKey(error) }]);
    } finally {
      running.current = false;
      if (current === generation.current) { setBusy(false); setProgress(null); }
    }
  }
  async function remove(audioId: string) {
    if (running.current) return;
    running.current = true;
    setBusy(true);
    setErrors([]);
    const current = generation.current;
    try {
      const entry = await onFlush();
      if (current !== generation.current) return;
      if (!entry || entry.id !== entryId) throw new Error("audio_stale");
      const result = await backend.removeAudio({ entryId, audioId, expectedRevision: entry.revision });
      if (current !== generation.current) return;
      onEntryMutated(result.entry);
      setItems((value) => value.filter((item) => item.id !== audioId));
    } catch (error) {
      if (current === generation.current) setErrors([{ name: "", key: errorKey(error) }]);
    } finally {
      running.current = false;
      if (current === generation.current) setBusy(false);
    }
  }
  return <div className="audio-editor">
    <div className="subsection-heading"><div><h4>{t("audio.title")}</h4><small>{t("audio.help")}</small></div><Button type="button" size="small" disabled={busy} onClick={() => void add()}><Music2 size={14} />{t("audio.add")}</Button></div>
    {progress && <p role="status">{t("audio.importing", progress)}</p>}
    {errors.map((error, index) => <p key={index} className="image-error" role="alert">{error.name && `${error.name}: `}{t(error.key, { defaultValue: t("audio.failed") })}</p>)}
    {items.map((item) => <AudioRow key={item.id} item={item} busy={busy} onRemove={() => void remove(item.id)} />)}
  </div>;
}

let activePlayer: HTMLAudioElement | null = null;
let playbackRequest = 0;
function AudioRow({ item, busy, onRemove }: { item: AudioAttachment; busy: boolean; onRemove(): void }) {
  const { t } = useTranslation();
  const player = useRef<HTMLAudioElement>(null);
  const url = useRef<string | null>(null);
  const alive = useRef(true);
  const [loading, setLoading] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(0);
  const [error, setError] = useState(false);
  useEffect(() => {
    alive.current = true;
    const element = player.current;
    return () => {
      alive.current = false;
      element?.pause();
      if (activePlayer === element) { activePlayer = null; playbackRequest++; }
      if (url.current) { URL.revokeObjectURL(url.current); url.current = null; }
      element?.removeAttribute("src");
    };
  }, []);
  async function toggle() {
    const element = player.current;
    if (!element || loading) return;
    if (!element.paused) { element.pause(); return; }
    const request = ++playbackRequest;
    activePlayer?.pause();
    activePlayer = element;
    setError(false);
    setLoading(true);
    try {
      if (!url.current) {
        const content = await backend.loadAudio(item.id);
        if (!alive.current || request !== playbackRequest) return;
        const bytes = Uint8Array.from(atob(content.dataBase64), (value) => value.charCodeAt(0));
        url.current = URL.createObjectURL(new Blob([bytes], { type: content.mimeType }));
        element.src = url.current;
      }
      if (alive.current && request === playbackRequest) await element.play();
    } catch {
      if (alive.current && request === playbackRequest) setError(true);
    } finally {
      if (alive.current) setLoading(false);
    }
  }
  const name = item.originalFilename;
  return <div className="audio-row">
    <audio ref={player} preload="none" onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => setPlaying(false)} onTimeUpdate={() => setPosition(player.current?.currentTime ?? 0)} onError={() => { setPlaying(false); setError(true); }} />
    <div className="audio-file"><span title={name}>{name}</span><small>{time(item.durationMs / 1000)} · {(item.byteSize / 1024).toFixed(1)} KiB</small></div>
    <Button type="button" size="icon" variant="ghost" disabled={loading} onClick={() => void toggle()} aria-label={t(playing ? "audio.pause" : "audio.play", { name })}>{playing ? <Pause size={15} /> : <Play size={15} />}</Button>
    <input type="range" min={0} max={item.durationMs / 1000} step={0.1} value={position} disabled={!url.current} aria-label={t("audio.seek", { name })} onChange={(event) => { const seconds = Number(event.target.value); if (player.current) player.current.currentTime = seconds; setPosition(seconds); }} />
    <span className="audio-time">{time(position)}</span>
    <Button type="button" size="icon" variant="danger" disabled={busy} onClick={onRemove} aria-label={t("audio.remove", { name })}><Trash2 size={14} /></Button>
    {loading && <span role="status">{t("common.loading")}</span>}
    {error && <span className="image-error" role="alert">{t("audio.playFailed")}</span>}
  </div>;
}
function time(seconds: number) { return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`; }
function errorKey(error: unknown) {
  if (error instanceof CommandError) return `error.${error.code}`;
  if (error instanceof Error && error.message === "audio_stale") return "error.audio_stale";
  return "audio.failed";
}
