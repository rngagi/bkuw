import * as Dialog from "@radix-ui/react-dialog";
import { ExternalLink, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type {
  CorpusPartOfSpeech,
  ExportKind,
  ExportPreview,
  ExportResult,
  ExportSettings,
  FontPackStatus,
  ProjectSnapshot,
  TexEngineStatus,
} from "../../types/domain";

interface Props {
  open: boolean;
  snapshot: ProjectSnapshot;
  onOpenChange(open: boolean): void;
  onFlush(): Promise<unknown>;
  onSetAnalysisLanguage(language: "zh-TW" | "en" | null): Promise<void>;
  onNavigateEntry(id: string): void;
}

const corpusParts: CorpusPartOfSpeech[] = [
  "noun", "verb", "adjective", "adverb", "pronoun", "particle", "other",
];
const fontPresets = ["auto", "charisSil", "notoSerif", "notoSerifCjkTc"] as const;

interface ExportError {
  message: string;
  diagnosticPath?: string;
}

function cloneSettings(settings: ExportSettings): ExportSettings {
  return JSON.parse(JSON.stringify(settings)) as ExportSettings;
}

export function ExportDialog({ open, snapshot, onOpenChange, onFlush, onSetAnalysisLanguage, onNavigateEntry }: Props) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<ExportKind>("corpusCsv");
  const [settings, setSettings] = useState<ExportSettings>(() => cloneSettings(snapshot.exportSettings));
  const [analysisLanguage, setAnalysisLanguage] = useState<"zh-TW" | "en" | null>(snapshot.project.analysisLanguage);
  const [preview, setPreview] = useState<ExportPreview | null>(null);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [engine, setEngine] = useState<TexEngineStatus | null>(null);
  const [fontPacks, setFontPacks] = useState<FontPackStatus[]>([]);
  const [installingPack, setInstallingPack] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ExportError | null>(null);
  const [pendingDestination, setPendingDestination] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setSettings(cloneSettings(snapshot.exportSettings));
    setAnalysisLanguage(snapshot.project.analysisLanguage);
    setPreview(null);
    setResult(null);
    setError(null);
    setPendingDestination(null);
    void backend.detectXeLatex().then(setEngine).catch(() => setEngine(null));
    void backend.listFontPacks().then(setFontPacks).catch(() => setFontPacks([]));
  }, [open, snapshot]);

  function patchLatex(value: Partial<ExportSettings["latex"]>) {
    setSettings((current) => ({ ...current, latex: { ...current.latex, ...value } }));
    setPreview(null);
    setResult(null);
  }

  async function makePreview() {
    setBusy(true);
    setError(null);
    try {
      await onFlush();
      if (analysisLanguage !== snapshot.project.analysisLanguage) {
        await onSetAnalysisLanguage(analysisLanguage);
      }
      const saved = await backend.saveExportSettings(settings);
      setSettings(saved);
      const nextPreview = await backend.previewExport(kind);
      setPreview(nextPreview);
      if (nextPreview.requiredFontPacks.length > 0) {
        setFontPacks((current) => mergeFontPacks(current, nextPreview.requiredFontPacks));
      }
      setResult(null);
    } catch (value) {
      setError({ message: value instanceof CommandError ? t(`error.${value.code}`) : t("error.generic") });
    } finally {
      setBusy(false);
    }
  }

  async function installAndRetry(packId: string) {
    setInstallingPack(packId);
    setError(null);
    try {
      const installed = await backend.installFontPack(packId);
      setFontPacks((current) => mergeFontPacks(current, [installed]));
      if (preview) {
        const nextPreview = await backend.previewExport(kind);
        setPreview(nextPreview);
        setFontPacks((current) => mergeFontPacks(current, nextPreview.requiredFontPacks));
      }
    } catch (value) {
      setError({ message: value instanceof CommandError ? t(`error.${value.code}`) : t("error.generic") });
    } finally {
      setInstallingPack(null);
    }
  }

  async function runExport(overwrite = false, destination?: string) {
    if (!preview) return;
    setBusy(true);
    setError(null);
    let selected = destination;
    try {
      if (!selected) {
        selected = kind === "corpusCsv"
          ? await backend.chooseCsvDestination(`${snapshot.project.name}.csv`) ?? undefined
          : await backend.chooseFolder() ?? undefined;
      }
      if (!selected) return;
      const exported = await backend.exportProject({
        kind,
        destination: selected,
        snapshotToken: preview.snapshotToken,
        overwrite,
      });
      setPendingDestination(null);
      setResult(exported);
    } catch (value) {
      if (value instanceof CommandError && value.code === "export_filesystem" && value.details === "destination_exists" && selected) {
        setPendingDestination(selected);
      } else {
        setError({
          message: value instanceof CommandError ? t(`error.${value.code}`) : t("error.generic"),
          diagnosticPath: value instanceof CommandError
            && (value.code === "latex_compile" || value.code === "latex_timeout")
            && value.details
            ? value.details
            : undefined,
        });
      }
    } finally {
      setBusy(false);
    }
  }

  const blockers = preview?.issues.filter((issue) => issue.severity === "error") ?? [];
  const warnings = preview?.issues.filter((issue) => issue.severity === "warning") ?? [];

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content export-dialog">
          <div className="dialog-heading"><div><Dialog.Title>{t("export.title")}</Dialog.Title><Dialog.Description>{t("export.description")}</Dialog.Description></div><Dialog.Close asChild><Button size="icon" variant="ghost" aria-label={t("common.close")}><X size={17} /></Button></Dialog.Close></div>
          {error && <div className="error-banner export-error" role="alert">
            <p>{error.message}</p>
            {error.diagnosticPath && <div className="diagnostic-location">
              <strong>{t("export.diagnosticLogLocation")}</strong>
              <code>{error.diagnosticPath}</code>
              <small>{t("export.diagnosticLogHelp")}</small>
            </div>}
          </div>}

          {!result && <div className="export-body">
            <fieldset className="format-picker"><legend>{t("export.format")}</legend>
              {(["corpusCsv", "latex", "pdf"] as ExportKind[]).map((value) => <label key={value}><input type="radio" aria-label={value === "corpusCsv" ? "CSV" : value === "latex" ? "LaTeX" : "PDF"} checked={kind === value} onChange={() => { setKind(value); setPreview(null); }} />{t(`export.kind.${value}`)}</label>)}
            </fieldset>

            <section className="export-profile"><h3>{t("export.profile")}</h3>
              <label className="field"><span>{t("export.analysisLanguage")}</span><select value={analysisLanguage ?? ""} onChange={(event) => setAnalysisLanguage((event.target.value || null) as "zh-TW" | "en" | null)}><option value="">{t("common.none")}</option><option value="zh-TW">{t("locale.zhTW")}</option><option value="en">{t("locale.en")}</option></select><small>{kind === "corpusCsv" ? t("export.corpusRequiresZhTW") : t("export.analysisLanguageHelp")}</small></label>

              {kind === "corpusCsv" ? <div className="mapping-grid">
                <p className="section-help">{t("export.posMappingHelp")}</p>
                {snapshot.partOfSpeechOptions.map((part) => <label className="field" key={part}><span>{part}</span><select aria-label={part} value={settings.corpus.partOfSpeechMappings[part] ?? ""} onChange={(event) => setSettings((current) => ({ ...current, corpus: { partOfSpeechMappings: { ...current.corpus.partOfSpeechMappings, [part]: event.target.value as CorpusPartOfSpeech } } }))}><option value="">{t("common.none")}</option>{corpusParts.map((value) => <option value={value} key={value}>{t(`export.pos.${value}`)}</option>)}</select></label>)}
              </div> : <div className="export-latex-grid">
                <label className="field"><span>{t("export.latexTitle")}</span><input value={settings.latex.title} onChange={(event) => patchLatex({ title: event.target.value })} /></label>
                <label className="field"><span>{t("export.author")}</span><input value={settings.latex.author} onChange={(event) => patchLatex({ author: event.target.value })} /></label>
                <WritingSystemSelect label={t("export.headwordSystem")} value={settings.latex.headwordWritingSystemId} systems={snapshot.writingSystems} onChange={(value) => patchLatex({ headwordWritingSystemId: value })} />
                <WritingSystemSelect label={t("export.pronunciationSystem")} optional value={settings.latex.pronunciationWritingSystemId ?? ""} systems={snapshot.writingSystems} onChange={(value) => patchLatex({ pronunciationWritingSystemId: value || null })} />
                <WritingSystemSelect label={t("export.exampleSystem")} value={settings.latex.exampleWritingSystemId} systems={snapshot.writingSystems} onChange={(value) => patchLatex({ exampleWritingSystemId: value })} />
                <div className="field"><span>{t("export.dictionaryOrder")}</span><output>{t("export.dictionaryOrderHelp")}</output></div>
                <label className="field"><span>{t("export.reverseIndex")}</span><select value={settings.latex.reverseIndex} onChange={(event) => patchLatex({ reverseIndex: event.target.value as ExportSettings["latex"]["reverseIndex"] })}><option value="gloss">{t("export.gloss")}</option><option value="none">{t("common.none")}</option></select></label>
                <label className="field"><span>{t("export.relatedEntries")}</span><select aria-label={t("export.relatedEntries")} value={settings.latex.relatedEntries} onChange={(event) => patchLatex({ relatedEntries: event.target.value as ExportSettings["latex"]["relatedEntries"] })}><option value="none">{t("common.none")}</option><option value="root">{t("export.relatedRoot")}</option><option value="base">{t("export.relatedBase")}</option><option value="both">{t("export.relatedBoth")}</option></select><small>{t("export.relatedEntriesHelp")}</small></label>
                {snapshot.writingSystems.map((system) => <label className="field" key={system.id}><span>{t("export.fontFor", { name: system.name })}</span>{system.type === "phonemic" || system.type === "phonetic" ? <output>{t("export.ipaFixedFont")}</output> : <select value={settings.latex.fontPresets[system.id] ?? "auto"} onChange={(event) => patchLatex({ fontPresets: { ...settings.latex.fontPresets, [system.id]: event.target.value as typeof fontPresets[number] } })}>{fontPresets.map((preset) => <option value={preset} key={preset}>{t(`export.font.${preset}`)}</option>)}</select>}</label>)}
                {kind === "pdf" && <p className="engine-status">{engine?.available ? t("export.xelatexFound", { path: engine.path }) : t("export.xelatexMissing")}</p>}
              </div>}
            </section>

            {kind !== "corpusCsv" && <section className="font-pack-manager"><h3>{t("export.fontPacks")}</h3><p className="section-help">{t("export.fontPacksHelp")}</p><ul>{fontPacks.map((pack) => <li key={pack.id}><div><strong>{t(`export.fontPack.${pack.id}`)}</strong><small>{t(`export.fontState.${pack.state}`)} · {pack.version}{pack.mandatory ? ` · ${t("export.mandatory")}` : ""}</small></div>{pack.state !== "installed" && <Button disabled={installingPack !== null} onClick={() => void installAndRetry(pack.id)}>{installingPack === pack.id ? t("export.downloadingFont") : t("export.downloadAndRetry")}</Button>}</li>)}</ul></section>}

            {preview && <section className="export-preview" aria-live="polite"><h3>{t("export.preview")}</h3><p>{t("export.rowsReady", { count: preview.rowCount })}</p><p>{t("export.issueCounts", { errors: blockers.length, warnings: warnings.length })}</p>
              {preview.issues.length > 0 && <ul>{preview.issues.map((issue, index) => { const label = t(`export.issue.${issue.code}`, { defaultValue: issue.code, font: issue.details ? t(`export.fontPack.${issue.details}`, { defaultValue: issue.details }) : "" }); return <li key={`${issue.code}-${index}`} className={issue.severity}>{issue.entryId ? <button type="button" className="inline-link" onClick={() => onNavigateEntry(issue.entryId!)}>{label}</button> : label}</li>; })}</ul>}
              {(preview.omitted.examples > 0 || preview.omitted.exampleForms > 0 || preview.omitted.baseRelations > 0) && <p>{t("export.omitted", preview.omitted)}</p>}
            </section>}

            {pendingDestination && <div className="overwrite-confirm" role="alertdialog" aria-label={t("export.overwriteTitle")}><strong>{t("export.overwriteTitle")}</strong><p>{t("export.overwriteBody")}</p><div className="dialog-actions"><Button onClick={() => setPendingDestination(null)}>{t("common.cancel")}</Button><Button variant="danger" onClick={() => void runExport(true, pendingDestination)}>{t("export.overwrite")}</Button></div></div>}
            <div className="dialog-actions"><Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>{!preview ? <Button variant="primary" disabled={busy} onClick={() => void makePreview()}>{busy ? t("common.loading") : t("export.preview")}</Button> : <Button variant="primary" disabled={busy || blockers.length > 0} onClick={() => void runExport()}>{busy ? t("common.loading") : t("export.chooseAndExport")}</Button>}</div>
          </div>}

          {result && <section className="export-result"><h3>{t("export.complete")}</h3><p>{t("export.exportedRows", { count: result.rowCount })}</p>{result.csvPath && <code>{result.csvPath}</code>}{result.latexDirectory && <code>{result.latexDirectory}</code>}{result.zipPath && <code>{result.zipPath}</code>}{result.pdfPath && <code>{result.pdfPath}</code>}{result.pdfStatus === "xeLatexMissing" && <><p>{t("export.missingResult")}</p><p>{t("export.overleafSteps")}</p><div className="inline-field"><Button onClick={() => void backend.openOverleaf()}><ExternalLink size={15} />{t("export.openOverleaf")}</Button><Button variant="ghost" onClick={() => void backend.openOverleafCompilerHelp()}>{t("export.compilerHelp")}</Button></div></>}<div className="dialog-actions"><Button variant="primary" onClick={() => onOpenChange(false)}>{t("common.close")}</Button></div></section>}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function mergeFontPacks(current: FontPackStatus[], updates: FontPackStatus[]) {
  const merged = new Map(current.map((pack) => [pack.id, pack]));
  for (const pack of updates) merged.set(pack.id, pack);
  return Array.from(merged.values());
}

function WritingSystemSelect({ label, value, systems, optional = false, onChange }: { label: string; value: string; systems: ProjectSnapshot["writingSystems"]; optional?: boolean; onChange(value: string): void }) {
  const { t } = useTranslation();
  return <label className="field"><span>{label}</span><select value={value} onChange={(event) => onChange(event.target.value)}>{optional && <option value="">{t("common.none")}</option>}{systems.map((system) => <option key={system.id} value={system.id}>{system.name}</option>)}</select></label>;
}
