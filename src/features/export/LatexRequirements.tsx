import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type { LatexEnvironment, LatexInstallProgress } from "../../types/domain";

interface Props {
  revision: number;
  onStatus(status: LatexEnvironment | null): void;
  onBusy(busy: boolean): void;
}

export function LatexRequirements({ revision, onStatus, onBusy }: Props) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<LatexEnvironment | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<LatexInstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [guide, setGuide] = useState<string | null>(null);
  const generation = useRef(0);

  const check = useCallback(async () => {
    const request = ++generation.current;
    setChecking(true);
    setError(null);
    setStatus(null);
    onStatus(null);
    try {
      const next = await backend.checkLatexEnvironment();
      if (request === generation.current) { setStatus(next); onStatus(next); }
    } catch (value) {
      if (request === generation.current) setError(value instanceof CommandError ? value.code : "latex_environment");
    } finally {
      if (request === generation.current) setChecking(false);
    }
  }, [onStatus]);

  useEffect(() => {
    void check();
    return () => { generation.current++; };
  }, [check, revision]);

  useEffect(() => {
    if (progress?.phase !== "waiting") return;
    const focused = () => { if (!checking) void check(); };
    window.addEventListener("focus", focused);
    return () => window.removeEventListener("focus", focused);
  }, [progress?.phase, checking, check]);

  async function install() {
    setInstalling(true); onBusy(true); setError(null);
    setProgress({ phase: "downloading", downloadedBytes: 0, totalBytes: null });
    try { await backend.installLatex(setProgress); }
    catch (value) { setProgress(null); setError(value instanceof CommandError ? value.code : "latex_install_download"); }
    finally { setInstalling(false); onBusy(false); }
  }

  async function saveGuide() {
    try {
      const destination = await backend.chooseFolder();
      if (destination) setGuide(await backend.saveLatexInstallGuide(destination));
    } catch (value) { setError(value instanceof CommandError ? value.code : "latex_install_filesystem"); }
  }

  function help(topic: "texlive" | "mactex" | "miktex") {
    void backend.openExportHelp(topic).catch(() => setError("export_help"));
  }

  return <section className="latex-requirements" aria-label={t("export.wizard.requirements")}>
    <h3>{t("export.wizard.localEnvironment")}</h3>
    <p role="status">{t(`export.environment.${checking ? "checking" : status?.state ?? (error ? "checkFailed" : "checking")}`)}</p>
    {status?.version && <p>{status.version}</p>}
    {status?.path && <code>{status.path}</code>}
    {!!status?.missingFiles.length && <p>{status.missingFiles.join(", ")}</p>}
    {status?.diagnosticPath && <p><strong>{t("export.diagnosticLogLocation")}</strong><code>{status.diagnosticPath}</code></p>}
    {error && <p role="alert">{t(`error.${error}`)}</p>}
    {status?.state === "missing" && <>
      <p>{t("export.wizard.installHelp")}</p>
      <Button disabled={installing || checking || progress?.phase === "waiting"} onClick={() => void install()}>{t("export.wizard.install")}</Button>
    </>}
    {status && !["ready", "missing", "fontsMissing"].includes(status.state) && <>
      <p>{t("export.wizard.repairHelp")}</p>
      <div className="inline-field">{(["texlive", "mactex", "miktex"] as const).map((topic) => <Button key={topic} onClick={() => help(topic)}>{t(`export.help.${topic}`)}</Button>)}</div>
    </>}
    {progress && status?.state !== "ready" && <div role="status">
      <p>{t(`export.installer.${progress.phase}`)}</p>
      {progress.phase === "downloading" && <>
        <progress aria-label={t("export.installer.downloading")} value={progress.totalBytes ? progress.downloadedBytes : undefined} max={progress.totalBytes ?? undefined} />
        <p>{t("export.wizard.downloadBytes", { downloaded: (progress.downloadedBytes / 1048576).toFixed(1), total: progress.totalBytes ? (progress.totalBytes / 1048576).toFixed(1) : "?" })}</p>
        <Button onClick={() => void backend.cancelLatexDownload().catch(() => setError("latex_install_download"))}>{t("common.cancel")}</Button>
      </>}
      {progress.phase === "waiting" && <Button onClick={() => setProgress(null)}>{t("export.wizard.installerClosed")}</Button>}
    </div>}
    <div className="inline-field">
      <Button disabled={checking || installing} onClick={() => void check()}>{t("export.wizard.recheck")}</Button>
      <Button disabled={installing} onClick={() => void saveGuide()}>{t("export.wizard.saveGuide")}</Button>
    </div>
    {guide && <code>{guide}</code>}
  </section>;
}

export function OverleafHelp() {
  const { t } = useTranslation();
  const [error, setError] = useState(false);
  return <section className="overleaf-help">
    <h3>{t("export.wizard.overleafGuide")}</h3>
    <p>{t("export.wizard.manualUpload")}</p>
    <ol>{(["upload", "compiler", "main", "compile", "download"] as const).map((topic) => <li key={topic}>
      <p>{t(`export.help.${topic}Step`)}</p>
      <Button variant="ghost" onClick={() => void backend.openExportHelp(topic).catch(() => setError(true))}>{t(`export.help.${topic}`)}</Button>
    </li>)}</ol>
    <Button onClick={() => void backend.openOverleaf().catch(() => setError(true))}>{t("export.openOverleaf")}</Button>
    {error && <p role="alert">{t("error.export_help")}</p>}
  </section>;
}
