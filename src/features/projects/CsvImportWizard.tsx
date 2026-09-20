import { ArrowLeft, ArrowRight, FileSpreadsheet, Plus, Trash2, X } from "lucide-react";
import type { TFunction } from "i18next";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { FontManagerButton, type FontStatus } from "../fonts/FontManagerButton";
import { backend, CommandError } from "../../lib/tauri";
import { createId, type CsvColumnMapping, type CsvDelimiter, type CsvImportPreview, type CsvImportResult, type CsvInspection, type CsvMappingTarget, type CsvPreviewIssue, type CsvPreviewRequest, type ProjectSnapshot, type WritingSystem } from "../../types/domain";
import { LocaleSelect } from "./LocaleSelect";

interface Props {
  fontStatus?: FontStatus;
  onManageFonts?(): void;
  onCancel(): void;
  onProject(snapshot: ProjectSnapshot): void;
  onError(error: unknown): void;
}

const systemTypes = ["orthography", "romanization", "transliteration", "phonemic", "phonetic", "other"] as const;

function initialSystems(inspection: CsvInspection): WritingSystem[] {
  const primary: WritingSystem = { id: createId(), name: "Primary orthography", type: "orthography", scriptCode: null, languageTag: null, displayRole: "primary", sortOrder: 0, fontFamily: null, notes: null };
  const hasIpa = inspection.columns.some((column) => /^(ipa|phonetic|pronunciation)$/i.test(column.name));
  return hasIpa ? [primary, { id: createId(), name: "IPA", type: "phonetic", scriptCode: "Latn", languageTag: null, displayRole: null, sortOrder: 1, fontFamily: null, notes: null }] : [primary];
}

function targetKey(target: CsvMappingTarget): string {
  return "writingSystemId" in target ? `${target.kind}:${target.writingSystemId}` : target.kind;
}

function parseTarget(value: string): CsvMappingTarget {
  const [kind, writingSystemId] = value.split(":", 2);
  if (kind === "entryForm" || kind === "exampleForm") return { kind, writingSystemId };
  return { kind } as CsvMappingTarget;
}

function commandDetails(value?: string): Record<string, string | number> {
  if (!value) return {};
  try {
    const parsed: unknown = JSON.parse(value);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) return parsed as Record<string, string | number>;
  } catch { /* Use the translated fallback without exposing raw parser diagnostics. */ }
  return {};
}

function targetLabel(key: string, systems: WritingSystem[], t: TFunction): string {
  const [kind, writingSystemId] = key.split(":", 2);
  const systemName = systems.find((system) => system.id === writingSystemId)?.name ?? writingSystemId;
  if (kind === "entryForm") return t("csv.targetEntryForm", { name: systemName });
  if (kind === "exampleForm") return t("csv.targetExampleForm", { name: systemName });
  const labels: Record<string, string> = {
    entryNotes: t("csv.targetEntryNotes"), rootFallback: t("csv.targetRoots"),
    senseGloss: t("entry.gloss"), senseDefinition: t("entry.definition"),
    partOfSpeech: t("entry.partOfSpeech"), semanticDomain: t("entry.semanticDomain"),
    exampleTranslation: t("entry.translation"), exampleNotes: t("entry.exampleNotes"),
  };
  return labels[kind] ?? key;
}

function previewIssueLabel(issue: CsvPreviewIssue, inspection: CsvInspection, systems: WritingSystem[], isZh: boolean, t: TFunction): string {
  const separator = isZh ? "、" : ", ";
  const columns = [...new Set(issue.columnIndices)].map((index) => {
    const name = inspection.columns.find((column) => column.index === index)?.name ?? `#${index + 1}`;
    return isZh ? `「${name}」` : `“${name}”`;
  }).join(separator);
  return t(`csv.issue.${issue.code}`, {
    columns,
    details: issue.details ?? "",
    target: issue.details ? targetLabel(issue.details, systems, t) : "",
    defaultValue: issue.code,
  });
}

function suggestedMappings(inspection: CsvInspection, systems: WritingSystem[]): CsvColumnMapping[] {
  const primary = systems.find((system) => system.displayRole === "primary") ?? systems[0];
  const ipa = systems.find((system) => system.type === "phonetic" || system.type === "phonemic");
  return inspection.columns.map((column) => {
    const key = column.name.trim().toLowerCase().replace(/[\s-]+/g, "_");
    let target: CsvMappingTarget = { kind: "ignore" };
    if (["form", "headword", "word", "entry", "lemma"].includes(key)) target = { kind: "entryForm", writingSystemId: primary.id };
    else if (["ipa", "phonetic", "pronunciation"].includes(key) && ipa) target = { kind: "entryForm", writingSystemId: ipa.id };
    else if (["gloss", "gloss_zh", "meaning"].includes(key)) target = { kind: "senseGloss" };
    else if (["definition", "gloss_en"].includes(key)) target = { kind: "senseDefinition" };
    else if (["part_of_speech", "pos"].includes(key)) target = { kind: "partOfSpeech" };
    else if (["semantic_domain", "semantic_category", "domain"].includes(key)) target = { kind: "semanticDomain" };
    else if (["example", "example_form", "sentence"].includes(key)) target = { kind: "exampleForm", writingSystemId: primary.id };
    else if (["example_translation", "example_translation_zh", "translation"].includes(key)) target = { kind: "exampleTranslation" };
    else if (["example_notes"].includes(key)) target = { kind: "exampleNotes" };
    else if (["word_root", "root", "roots"].includes(key)) target = { kind: "rootFallback" };
    else if (["entry_notes", "notes"].includes(key)) target = { kind: "entryNotes" };
    return { columnIndex: column.index, target };
  });
}

export function CsvImportWizard({ fontStatus, onManageFonts, onCancel, onProject, onError }: Props) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState(0);
  const [inspection, setInspection] = useState<CsvInspection | null>(null);
  const [delimiter, setDelimiter] = useState<CsvDelimiter>("comma");
  const [name, setName] = useState("");
  const [languageName, setLanguageName] = useState("");
  const [languageCode, setLanguageCode] = useState("");
  const [analysisLanguage, setAnalysisLanguage] = useState<"zh-TW" | "en" | "">("");
  const [systems, setSystems] = useState<WritingSystem[]>([]);
  const [mappings, setMappings] = useState<CsvColumnMapping[]>([]);
  const [groups, setGroups] = useState<number[][]>([]);
  const [excludedRows, setExcludedRows] = useState<number[]>([]);
  const [rootDelimiter, setRootDelimiter] = useState(";");
  const [parentDir, setParentDir] = useState("");
  const [preview, setPreview] = useState<CsvImportPreview | null>(null);
  const [result, setResult] = useState<CsvImportResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [localError, setLocalError] = useState("");

  function showCsvError(error: unknown) {
    if (error instanceof CommandError) {
      setLocalError(t(`error.${error.code}`, { ...commandDetails(error.details), defaultValue: error.message }));
    } else {
      onError(error);
    }
  }

  const request = useMemo<CsvPreviewRequest | null>(() => inspection ? {
    sourcePath: inspection.sourcePath,
    delimiter,
    project: { parentDir: parentDir || ".", name, languageName: languageName.trim() || null, languageCode: languageCode.trim() || null, analysisLanguage: analysisLanguage || null, writingSystems: systems.map((system, index) => ({ ...system, sortOrder: index })) },
    mappings,
    groups: groups.map((rowIndices) => ({ rowIndices })),
    excludedRows,
    rootDelimiter: rootDelimiter || ";",
  } : null, [analysisLanguage, delimiter, excludedRows, groups, inspection, languageCode, languageName, mappings, name, parentDir, rootDelimiter, systems]);

  async function chooseFile() {
    try {
      const path = await backend.chooseCsvFile();
      if (!path) return;
      setBusy(true);
      const next = await backend.inspectCsv(path);
      const nextSystems = initialSystems(next);
      setInspection(next); setDelimiter(next.delimiter); setName(next.fileName.replace(/\.[^.]+$/, ""));
      setSystems(nextSystems); setMappings(suggestedMappings(next, nextSystems)); setGroups([]); setExcludedRows([]); setPreview(null); setLocalError("");
    } catch (error) { showCsvError(error); } finally { setBusy(false); }
  }

  async function overrideDelimiter(next: CsvDelimiter) {
    if (!inspection) return;
    try {
      setBusy(true);
      const nextInspection = await backend.inspectCsv(inspection.sourcePath, next);
      setInspection(nextInspection); setDelimiter(next); setMappings(suggestedMappings(nextInspection, systems)); setGroups([]); setPreview(null);
    } catch (error) { showCsvError(error); } finally { setBusy(false); }
  }

  function patchSystem(index: number, patch: Partial<WritingSystem>) {
    setSystems((current) => current.map((system, itemIndex) => itemIndex === index ? { ...system, ...patch } : system));
  }

  function setPrimary(index: number) {
    setSystems((current) => current.map((system, itemIndex) => ({ ...system, displayRole: itemIndex === index ? "primary" : system.displayRole === "primary" ? null : system.displayRole })));
  }

  async function loadPreview(useSuggestedGroups = false) {
    if (!request) return;
    try {
      setBusy(true); setLocalError("");
      const next = await backend.previewCsvImport(useSuggestedGroups ? { ...request, groups: [] } : request);
      setPreview(next); setGroups(next.groups.map((group) => group.rowIndices)); setStep(3);
    } catch (error) { showCsvError(error); } finally { setBusy(false); }
  }

  async function refreshGroups(nextGroups: number[][], nextExcluded = excludedRows) {
    if (!request) return;
    try {
      setBusy(true);
      const next = await backend.previewCsvImport({ ...request, groups: nextGroups.map((rowIndices) => ({ rowIndices })), excludedRows: nextExcluded });
      setGroups(next.groups.map((group) => group.rowIndices)); setExcludedRows(nextExcluded); setPreview(next);
    } catch (error) { showCsvError(error); } finally { setBusy(false); }
  }

  function toggleExcluded(row: number) {
    const nextExcluded = excludedRows.includes(row) ? excludedRows.filter((value) => value !== row) : [...excludedRows, row];
    void refreshGroups([], nextExcluded);
  }

  async function chooseDestination() {
    const folder = await backend.chooseFolder();
    if (folder) setParentDir(folder);
  }

  async function importProject() {
    if (!request || !parentDir) return;
    try {
      setBusy(true); setLocalError("");
      const finalRequest = { ...request, project: { ...request.project, parentDir } };
      const finalPreview = await backend.previewCsvImport(finalRequest);
      setPreview(finalPreview);
      if (finalPreview.blockingErrorCount) { setLocalError(t("csv.resolveErrors")); setStep(3); return; }
      const imported = await backend.createProjectFromCsv(finalRequest, finalPreview.previewToken);
      setResult(imported); setStep(5);
    } catch (error) { showCsvError(error); } finally { setBusy(false); }
  }

  useEffect(() => {
    if (step === 2 && inspection && mappings.length === 0) setMappings(suggestedMappings(inspection, systems));
  }, [inspection, mappings.length, step, systems]);

  const targetOptions = systems.flatMap((system) => [
    { value: `entryForm:${system.id}`, label: t("csv.targetEntryForm", { name: system.name }) },
    { value: `exampleForm:${system.id}`, label: t("csv.targetExampleForm", { name: system.name }) },
  ]);

  return (
    <main className="csv-wizard">
      <header className="start-header"><strong>bkuw</strong><div className="header-actions"><div className="header-preferences"><LocaleSelect />{fontStatus && onManageFonts && <FontManagerButton status={fontStatus} onClick={onManageFonts} />}</div><Button size="small" variant="ghost" onClick={onCancel}><X size={16} />{t("common.close")}</Button></div></header>
      <div className="csv-wizard-shell">
        <div className="csv-wizard-heading"><div><p className="eyebrow">{t("csv.step", { current: Math.min(step + 1, 5), total: 5 })}</p><h1>{t("csv.title")}</h1></div>{inspection && <span className="csv-file-name"><FileSpreadsheet size={16} />{inspection.fileName}</span>}</div>
        {localError && <p className="error-banner" role="alert">{localError}</p>}

        {step === 0 && <section className="csv-step stack"><h2>{t("csv.chooseFile")}</h2><p>{t("csv.chooseFileHelp")}</p><Button variant="primary" onClick={() => void chooseFile()} disabled={busy}><FileSpreadsheet size={17} />{inspection ? t("csv.chooseAnother") : t("csv.browse")}</Button>{inspection && <><div className="two-columns"><label className="field"><span>{t("csv.delimiter")}</span><select value={delimiter} onChange={(event) => void overrideDelimiter(event.target.value as CsvDelimiter)}><option value="comma">{t("csv.comma")}</option><option value="tab">{t("csv.tab")}</option><option value="semicolon">{t("csv.semicolon")}</option></select></label><div className="field"><span>{t("csv.detected")}</span><output>{t("csv.rowsColumns", { rows: inspection.rowCount, columns: inspection.columns.length })}</output></div></div>{inspection.profile && <p className="info-banner">{t("csv.rngagiDetected")}</p>}</>}</section>}

        {step === 1 && <section className="csv-step stack"><h2>{t("csv.projectWritingSystems")}</h2><div className="two-columns"><label className="field"><span>{t("start.projectName")}</span><input value={name} onChange={(event) => setName(event.target.value)} /></label><label className="field"><span>{t("settings.analysisLanguage")}</span><select value={analysisLanguage} onChange={(event) => setAnalysisLanguage(event.target.value as typeof analysisLanguage)}><option value="">{t("common.none")}</option><option value="zh-TW">繁體中文（台灣）</option><option value="en">English</option></select></label><label className="field"><span>{t("start.languageName")}</span><input value={languageName} onChange={(event) => setLanguageName(event.target.value)} /></label><label className="field"><span>{t("start.languageCode")}</span><input value={languageCode} maxLength={3} onChange={(event) => setLanguageCode(event.target.value.toLowerCase().replace(/[^a-z]/g, ""))} /></label></div><div className="section-heading"><h3>{t("settings.writingSystems")}</h3><Button size="small" onClick={() => setSystems((current) => [...current, { id: createId(), name: "", type: "orthography", scriptCode: null, languageTag: null, displayRole: null, sortOrder: current.length, fontFamily: null, notes: null }])}><Plus size={15} />{t("settings.addWritingSystem")}</Button></div>{systems.map((system, index) => <div className="csv-system-row" key={system.id}><label className="field"><span>{t("settings.name")}</span><input value={system.name} onChange={(event) => patchSystem(index, { name: event.target.value })} /></label><label className="field"><span>{t("settings.type")}</span><select value={system.type} onChange={(event) => patchSystem(index, { type: event.target.value as WritingSystem["type"] })}>{systemTypes.map((type) => <option key={type} value={type}>{t(`settings.type_${type}`)}</option>)}</select></label><label className="field"><span>{t("settings.scriptCode")}</span><input value={system.scriptCode ?? ""} onChange={(event) => patchSystem(index, { scriptCode: event.target.value || null })} /></label><label className="field"><span>{t("settings.languageTag")}</span><input value={system.languageTag ?? ""} onChange={(event) => patchSystem(index, { languageTag: event.target.value || null })} /></label><label className="checkbox-field"><input type="radio" name="csv-primary" checked={system.displayRole === "primary"} onChange={() => setPrimary(index)} />{t("settings.primary")}</label><Button size="icon" variant="ghost" aria-label={t("common.remove")} disabled={systems.length === 1} onClick={() => setSystems((current) => current.filter((_, itemIndex) => itemIndex !== index))}><Trash2 size={15} /></Button></div>)}</section>}

        {step === 2 && inspection && <section className="csv-step stack"><h2>{t("csv.mapColumns")}</h2><p>{t("csv.mapHelp")}</p><div className="csv-mapping-table">{inspection.columns.map((column) => { const mapping = mappings.find((item) => item.columnIndex === column.index) ?? { columnIndex: column.index, target: { kind: "ignore" } as CsvMappingTarget }; return <div className="csv-mapping-row" key={column.index}><div><strong>{column.name}</strong><small>{column.samples.join(" · ") || t("csv.emptySamples")}</small></div><ArrowRight size={16} /><select aria-label={t("csv.mapColumn", { name: column.name })} value={targetKey(mapping.target)} onChange={(event) => setMappings((current) => current.filter((item) => item.columnIndex !== column.index).concat({ columnIndex: column.index, target: parseTarget(event.target.value) }).sort((a, b) => a.columnIndex - b.columnIndex))}><option value="ignore">{t("csv.ignore")}</option><optgroup label={t("csv.entryTargets")}>{targetOptions.filter((option) => option.value.startsWith("entryForm")).map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}<option value="entryNotes">{t("csv.targetEntryNotes")}</option><option value="rootFallback">{t("csv.targetRoots")}</option></optgroup><optgroup label={t("csv.senseTargets")}><option value="senseGloss">{t("entry.gloss")}</option><option value="senseDefinition">{t("entry.definition")}</option><option value="partOfSpeech">{t("entry.partOfSpeech")}</option><option value="semanticDomain">{t("entry.semanticDomain")}</option></optgroup><optgroup label={t("csv.exampleTargets")}>{targetOptions.filter((option) => option.value.startsWith("exampleForm")).map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}<option value="exampleTranslation">{t("entry.translation")}</option><option value="exampleNotes">{t("entry.exampleNotes")}</option></optgroup></select></div>; })}</div><label className="field narrow-field"><span>{t("csv.rootDelimiter")}</span><input value={rootDelimiter} maxLength={1} onChange={(event) => setRootDelimiter(event.target.value)} /></label></section>}

        {step === 3 && preview && <section className="csv-step stack"><h2>{t("csv.groupValidate")}</h2><div className="csv-counts"><span>{t("csv.importEntries", { count: preview.importEntryCount })}</span><span>{t("csv.skipRows", { count: preview.skippedRowCount })}</span><span className={preview.blockingErrorCount ? "count-error" : ""}>{t("csv.errors", { count: preview.blockingErrorCount })}</span><span>{t("csv.warnings", { count: preview.warningCount })}</span></div>{preview.issues.length > 0 && <div className="csv-issues">{preview.issues.map((issue, index) => <p className={issue.severity} key={`${issue.code}-${index}`}>{previewIssueLabel(issue, inspection!, systems, i18n.resolvedLanguage === "zh-TW", t)}{issue.rowIndices.length ? ` (${t("csv.rows", { rows: issue.rowIndices.map((row) => row + 2).join(i18n.resolvedLanguage === "zh-TW" ? "、" : ", ") })})` : ""}</p>)}</div>}<div className="csv-groups">{preview.groups.map((group, groupIndex) => <div className={`csv-group ${group.blocked ? "blocked" : ""}`} key={group.rowIndices.join("-")}><div><strong>{group.primaryForm || t("workspace.untitled")}</strong><span>{t("csv.sourceRows", { rows: group.rowIndices.map((row) => row + 2).join(i18n.resolvedLanguage === "zh-TW" ? "、" : ", ") })}</span></div><div className="row-actions">{group.rowIndices.length > 1 && <Button size="small" onClick={() => { const [first, ...rest] = group.rowIndices; const next = [...groups]; next.splice(groupIndex, 1, [first], rest); void refreshGroups(next); }}>{t("csv.split")}</Button>}{groupIndex < groups.length - 1 && groups[groupIndex].at(-1)! + 1 === groups[groupIndex + 1][0] && <Button size="small" onClick={() => { const next = [...groups]; next.splice(groupIndex, 2, [...groups[groupIndex], ...groups[groupIndex + 1]]); void refreshGroups(next); }}>{t("csv.mergeNext")}</Button>}</div><div className="csv-group-rows">{group.rowIndices.map((row) => <label key={row}><input type="checkbox" checked={excludedRows.includes(row)} onChange={() => toggleExcluded(row)} />{t("csv.excludeRow", { row: row + 2 })}</label>)}</div></div>)}</div></section>}

        {step === 4 && <section className="csv-step stack"><h2>{t("csv.destination")}</h2><p>{t("csv.destinationHelp")}</p><label className="field"><span>{t("start.parentFolder")}</span><div className="inline-field"><input value={parentDir} readOnly /><Button onClick={() => void chooseDestination()}>{t("start.chooseFolder")}</Button></div></label><div className="info-banner">{parentDir && name ? `${parentDir}/${name}.bkuw` : t("csv.destinationPending")}</div></section>}

        {step === 5 && result && <section className="csv-step csv-success"><h2>{t("csv.complete")}</h2><p>{t("csv.completeBody", { entries: result.importedEntryCount, senses: result.importedSenseCount, skipped: result.skippedRowCount })}</p><Button variant="primary" onClick={() => onProject(result.snapshot)}>{t("csv.openImported")}</Button></section>}

        {step < 5 && <footer className="csv-footer"><Button variant="ghost" disabled={busy || step === 0} onClick={() => setStep((current) => Math.max(0, current - 1))}><ArrowLeft size={16} />{t("common.back", { defaultValue: "Back" })}</Button>{step === 0 && <Button variant="primary" disabled={!inspection || busy} onClick={() => setStep(1)}>{t("common.next", { defaultValue: "Next" })}<ArrowRight size={16} /></Button>}{step === 1 && <Button variant="primary" disabled={!name.trim() || systems.length === 0 || systems.filter((system) => system.displayRole === "primary").length !== 1 || systems.some((system) => !system.name.trim())} onClick={() => { setMappings(suggestedMappings(inspection!, systems)); setStep(2); }}>{t("common.next", { defaultValue: "Next" })}<ArrowRight size={16} /></Button>}{step === 2 && <Button variant="primary" disabled={busy} onClick={() => void loadPreview(true)}>{t("csv.preview")}<ArrowRight size={16} /></Button>}{step === 3 && <Button variant="primary" disabled={busy || preview?.blockingErrorCount !== 0} onClick={() => setStep(4)}>{t("common.next", { defaultValue: "Next" })}<ArrowRight size={16} /></Button>}{step === 4 && <Button variant="primary" disabled={busy || !parentDir} onClick={() => void importProject()}>{busy ? t("common.loading") : t("csv.import")}</Button>}</footer>}
      </div>
    </main>
  );
}
