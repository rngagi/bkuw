import { ImagePlus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { pngDataUrl, prepareSenseImage } from "../../lib/imageCompression";
import { backend, CommandError } from "../../lib/tauri";
import type { LexicalEntry, SenseImage } from "../../types/domain";

interface Props {
  entryId: string;
  senseId: string;
  onFlush(): Promise<LexicalEntry | undefined>;
  onEntryMutated(entry: LexicalEntry): void;
}

export function SenseImageEditor({ entryId, senseId, onFlush, onEntryMutated }: Props) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [images, setImages] = useState<SenseImage[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void backend.listSenseImages(senseId).then((loaded) => {
      if (active) setImages(loaded);
    }).catch(() => {
      if (active) setImages([]);
    });
    return () => { active = false; };
  }, [senseId]);

  async function add(files: FileList | null) {
    if (!files?.length) return;
    setBusy(true);
    setError(null);
    try {
      for (const file of Array.from(files)) {
        const prepared = await prepareSenseImage(file);
        const current = await onFlush();
        if (!current) throw new Error("image_processing");
        const result = await backend.attachSenseImage({
          entryId,
          senseId,
          expectedRevision: current.revision,
          originalFilename: prepared.originalFilename,
          pngBase64: prepared.pngBase64,
        });
        onEntryMutated(result.entry);
        if (result.image) setImages((value) => [...value, result.image!]);
      }
    } catch (value) {
      setError(errorKey(value));
    } finally {
      setBusy(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  }

  async function remove(imageId: string) {
    setBusy(true);
    setError(null);
    try {
      const current = await onFlush();
      if (!current) throw new Error("image_processing");
      const result = await backend.removeSenseImage({
        entryId,
        imageId,
        expectedRevision: current.revision,
      });
      onEntryMutated(result.entry);
      setImages((value) => value.filter((image) => image.id !== imageId));
    } catch (value) {
      setError(errorKey(value));
    } finally {
      setBusy(false);
    }
  }

  return <div className="sense-images">
    <div className="subsection-heading"><div><h4>{t("entry.images")}</h4><small>{t("entry.imagesHelp")}</small></div><Button type="button" size="small" disabled={busy} onClick={() => inputRef.current?.click()}><ImagePlus size={14} />{busy ? t("entry.processingImage") : t("entry.addImage")}</Button></div>
    <input ref={inputRef} className="sr-only" type="file" accept="image/png,image/jpeg,image/webp,.png,.jpg,.jpeg,.webp" multiple onChange={(event) => void add(event.target.files)} />
    {error && <p className="image-error" role="alert">{t(error, { defaultValue: t("error.image_processing") })}</p>}
    {images.length > 0 && <div className="sense-image-grid">{images.map((image) => <SenseImageCard key={image.id} image={image} busy={busy} onRemove={() => void remove(image.id)} />)}</div>}
  </div>;
}

function SenseImageCard({ image, busy, onRemove }: { image: SenseImage; busy: boolean; onRemove(): void }) {
  const { t } = useTranslation();
  const [preview, setPreview] = useState<
    { status: "loading" } | { status: "ready"; url: string } | { status: "error" }
  >({ status: "loading" });
  useEffect(() => {
    let active = true;
    setPreview({ status: "loading" });
    void backend.loadSenseImage(image.id).then((content) => {
      if (!active) return;
      setPreview({ status: "ready", url: pngDataUrl(content.dataBase64) });
    }).catch(() => {
      if (active) setPreview({ status: "error" });
    });
    return () => {
      active = false;
    };
  }, [image.id]);
  return <figure className="sense-image-card">
    {preview.status === "ready" && <img src={preview.url} alt={image.originalFilename} onError={() => setPreview({ status: "error" })} />}
    {preview.status === "loading" && <div className="image-placeholder">{t("common.loading")}</div>}
    {preview.status === "error" && <div className="image-placeholder image-preview-error" role="status">{t("entry.imagePreviewFailed")}</div>}
    <figcaption><span title={image.originalFilename}>{image.originalFilename}</span><small>{image.width}×{image.height} · {formatBytes(image.byteSize)}</small></figcaption>
    <Button type="button" size="icon" variant="danger" disabled={busy} onClick={onRemove} aria-label={t("entry.removeImage")}><Trash2 size={14} /></Button>
  </figure>;
}

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function errorKey(value: unknown) {
  if (value instanceof CommandError) return `error.${value.code}`;
  if (value instanceof Error && value.message.startsWith("image_")) return `error.${value.message}`;
  return "error.image_processing";
}
