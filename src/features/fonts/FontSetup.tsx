import { Download, RotateCcw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type { FontInstallProgress, FontPackStatus } from "../../types/domain";

interface Props {
  onContinue(): void;
  autoContinue?: boolean;
  canClose?: boolean;
}

export function FontSetup({ onContinue, autoContinue = true, canClose = false }: Props) {
  const { t } = useTranslation();
  const [packs, setPacks] = useState<FontPackStatus[] | null>(null);
  const [progress, setProgress] = useState<FontInstallProgress | null>(null);
  const [installing, setInstalling] = useState(false);
  const [failed, setFailed] = useState(false);
  const [error, setError] = useState("");

  async function refresh() {
    try {
      const statuses = await backend.listFontPacks();
      setPacks(statuses);
      if (autoContinue && statuses.every((pack) => pack.state === "installed")) onContinue();
    } catch (value) {
      setFailed(true);
      setError(value instanceof CommandError ? t(`error.${value.code}`) : t("error.generic"));
    }
  }

  useEffect(() => { void refresh(); }, []);

  const pending = packs?.filter((pack) => pack.state !== "installed") ?? [];
  const completed = packs?.filter((pack) => pack.state === "installed").length ?? 0;
  const total = packs?.length ?? 0;
  const activeFraction = progress?.totalBytes && progress.totalBytes > 0
    ? Math.min(1, progress.downloadedBytes / progress.totalBytes)
    : 0;
  const overall = total > 0 ? Math.min(total, completed + activeFraction) : 0;
  const activePack = useMemo(
    () => packs?.find((pack) => pack.id === progress?.packId),
    [packs, progress?.packId],
  );

  async function installAll() {
    if (pending.length === 0) { onContinue(); return; }
    setInstalling(true);
    setFailed(false);
    setError("");
    try {
      await backend.installFontPacks(pending.map((pack) => pack.id), (next) => {
        setProgress(next);
        if (next.phase === "installed") {
          setPacks((current) => current?.map((pack) => pack.id === next.packId
            ? { ...pack, state: "installed", installedBytes: next.downloadedBytes }
            : pack) ?? null);
        }
      });
      const statuses = await backend.listFontPacks();
      setPacks(statuses);
      setProgress(null);
      if (statuses.every((pack) => pack.state === "installed")) onContinue();
    } catch (value) {
      setFailed(true);
      setError(value instanceof CommandError ? t(`error.${value.code}`) : t("error.generic"));
      await refresh();
    } finally {
      setInstalling(false);
    }
  }

  return (
    <main className="font-setup-page">
      <section className="font-setup-content" aria-labelledby="font-setup-title">
        <div className="brand-mark" aria-hidden="true">b</div>
        <h1 id="font-setup-title">{t("fontSetup.title")}</h1>
        <p>{t("fontSetup.body")}</p>
        {error && <p className="error-banner" role="alert">{error}</p>}
        <progress aria-label={t("fontSetup.overallProgress")} max={Math.max(total, 1)} value={overall} />
        <p className="font-progress-label">
          {progress
            ? t(`fontSetup.phase.${progress.phase}`, {
              font: t(`export.fontPack.${progress.packId}`, { defaultValue: activePack?.id ?? progress.packId }),
              downloaded: progress.downloadedBytes,
              total: progress.totalBytes ?? "—",
            })
            : t("fontSetup.completed", { completed, total })}
        </p>
        <ul className="font-pack-list">
          {(packs ?? []).map((pack) => (
            <li key={pack.id}>
              <span>{t(`export.fontPack.${pack.id}`)}</span>
              <span>{t(`export.fontState.${pack.state}`)}</span>
            </li>
          ))}
        </ul>
        <div className="font-setup-actions">
          <Button variant="primary" onClick={() => void installAll()} disabled={installing || packs === null || pending.length === 0}>
            {installing ? <RotateCcw className="spin" size={16} /> : <Download size={16} />}
            {installing ? t("fontSetup.installing") : t("fontSetup.installAll")}
          </Button>
          {(failed || canClose) && <Button variant="ghost" onClick={onContinue}>{t(canClose ? "common.close" : "fontSetup.useOffline")}</Button>}
        </div>
      </section>
    </main>
  );
}
