import { getCurrentWindow } from "@tauri-apps/api/window";
import { Group, Panel, Separator } from "react-resizable-panels";
import { FileOutput, FolderX, ListOrdered, Plus, Search, Settings } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./components/ui/Button";
import { EntryEditor, type EntryEditorHandle } from "./features/entries/EntryEditor";
import { EntryList } from "./features/entries/EntryList";
import { ExportDialog } from "./features/export/ExportDialog";
import { LocaleSelect } from "./features/projects/LocaleSelect";
import { ProjectStart } from "./features/projects/ProjectStart";
import { SettingsDialog } from "./features/settings/SettingsDialog";
import { SortOrderDialog } from "./features/settings/SortOrderDialog";
import { installTauriZoomShortcuts } from "./lib/appZoom";
import { backend, CommandError } from "./lib/tauri";
import type { EntrySortSettings, EntrySummary, LexicalEntry, ManualSortItem, ProjectSnapshot, WritingSystem } from "./types/domain";

function prepareEntryForms(entry: LexicalEntry, writingSystems: WritingSystem[]): LexicalEntry {
  const known = new Set(writingSystems.map((system) => system.id));
  const unknown = entry.forms.filter((form) => !known.has(form.writingSystemId));
  const ordered = writingSystems.flatMap((system) => {
    const matches = entry.forms.filter((form) => form.writingSystemId === system.id);
    return matches.length ? matches : [{
      id: crypto.randomUUID(), writingSystemId: system.id, text: "", variantLabel: null,
      dialect: null, status: null, notes: null, sortOrder: 0,
    }];
  });
  return {
    ...entry,
    forms: [...ordered, ...unknown].map((form, sortOrder) => ({ ...form, sortOrder })),
  };
}

function ErrorBanner({ error, className = "", onClose }: {
  error: { key: string; detail?: string };
  className?: string;
  onClose(): void;
}) {
  const { t } = useTranslation();
  return (
    <div className={`error-banner app-error ${className}`} role="alert">
      <span>{t(error.key, { defaultValue: t("error.generic") })}</span>
      {error.detail && <details><summary>{t("common.details")}</summary><code>{error.detail}</code></details>}
      <button onClick={onClose} aria-label={t("common.close")}>×</button>
    </div>
  );
}

function App() {
  const { t } = useTranslation();
  const [snapshot, setSnapshot] = useState<ProjectSnapshot | null>(null);
  const [entries, setEntries] = useState<EntrySummary[]>([]);
  const [entry, setEntry] = useState<LexicalEntry | null>(null);
  const [search, setSearch] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [sortOrderOpen, setSortOrderOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [onboarding, setOnboarding] = useState(false);
  const [error, setError] = useState<{ key: string; detail?: string } | null>(null);
  const [deletedId, setDeletedId] = useState<string | null>(null);
  const [loadingEntry, setLoadingEntry] = useState(false);
  const editorRef = useRef<EntryEditorHandle>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  const showError = useCallback((value: unknown) => {
    if (value instanceof CommandError) setError({ key: `error.${value.code}`, detail: value.details });
    else setError({ key: "error.generic" });
  }, []);

  useEffect(() => installTauriZoomShortcuts(() => setError({ key: "error.zoom_failed" })), []);

  const refreshEntries = useCallback(async (query: string) => {
    try {
      const result = await backend.queryEntries(query);
      setEntries(result);
      setSnapshot((current) => current ? { ...current, entries: result } : current);
    } catch (value) { showError(value); }
  }, [showError]);

  useEffect(() => {
    if (!snapshot) return;
    const timer = setTimeout(() => void refreshEntries(search), 180);
    return () => clearTimeout(timer);
  }, [refreshEntries, search, snapshot?.project.id]);

  async function flush() {
    return editorRef.current?.flush();
  }

  async function selectEntry(id: string) {
    if (id === entry?.id) return;
    try {
      await flush();
      setLoadingEntry(true);
      setEntry(prepareEntryForms(await backend.loadEntry(id), snapshot?.writingSystems ?? []));
      setError(null);
    } catch (value) { showError(value); }
    finally { setLoadingEntry(false); }
  }

  async function createEntry() {
    try {
      await flush();
      const created = await backend.createEntry();
      setEntry(prepareEntryForms(created, snapshot?.writingSystems ?? []));
      await refreshEntries(search);
    } catch (value) { showError(value); }
  }

  async function saveEntry(draft: LexicalEntry) {
    const saved = await backend.saveEntry(draft);
    void refreshEntries(search);
    return saved;
  }

  async function deleteEntry() {
    if (!entry) return;
    try {
      const flushed = await flush();
      const current = flushed ?? entry;
      await backend.deleteEntry(current.id, current.revision);
      setDeletedId(current.id);
      setEntry(null);
      await refreshEntries(search);
    } catch (value) { showError(value); }
  }

  async function undoDelete() {
    if (!deletedId) return;
    try {
      const restored = await backend.restoreEntry(deletedId);
      setDeletedId(null);
      setEntry(prepareEntryForms(restored, snapshot?.writingSystems ?? []));
      await refreshEntries(search);
    } catch (value) { showError(value); }
  }

  async function closeProject() {
    try {
      await flush();
      await backend.closeProject();
      setSnapshot(null); setEntries([]); setEntry(null); setSearch(""); setDeletedId(null);
    } catch (value) { showError(value); }
  }

  async function saveSettings(request: { name: string; languageName: string | null; languageCode: string | null; analysisLanguage: "zh-TW" | "en" | null; description: string | null; writingSystems: WritingSystem[]; partOfSpeechOptions: string[]; semanticDomainOptions: string[]; entrySortSettings: EntrySortSettings; manualInitialization: "headings" | "none" | null }) {
    await flush();
    const { entrySortSettings, manualInitialization, ...projectRequest } = request;
    let updated = await backend.updateProjectSettings(projectRequest);
    updated = await backend.saveEntrySortSettings(entrySortSettings);
    if (manualInitialization) {
      const items: ManualSortItem[] = [];
      let section: string | null = null;
      for (const summary of updated.entries) {
        if (manualInitialization === "headings" && summary.sectionLabel && summary.sectionLabel !== section) {
          section = summary.sectionLabel;
          items.push({ kind: "heading", id: crypto.randomUUID(), label: section });
        }
        items.push({ kind: "entry", entryId: summary.id });
      }
      updated = await backend.saveManualSortLayout({ version: 1, items });
    }
    setSnapshot(updated); setEntries(updated.entries);
    if (entry) setEntry(prepareEntryForms(await backend.loadEntry(entry.id), updated.writingSystems));
  }

  async function saveManualSortLayout(layout: ProjectSnapshot["manualSortLayout"]) {
    const updated = await backend.saveManualSortLayout(layout);
    setSnapshot(updated); setEntries(updated.entries);
  }

  async function setAnalysisLanguage(analysisLanguage: "zh-TW" | "en" | null) {
    if (!snapshot) return;
    const updated = await backend.updateProjectSettings({
      name: snapshot.project.name,
      languageName: snapshot.project.languageName,
      languageCode: snapshot.project.languageCode,
      analysisLanguage,
      description: snapshot.project.description,
      writingSystems: snapshot.writingSystems,
      partOfSpeechOptions: snapshot.partOfSpeechOptions,
      semanticDomainOptions: snapshot.semanticDomainOptions,
    });
    setSnapshot(updated);
    setEntries(updated.entries);
  }

  useEffect(() => {
    function shortcut(event: KeyboardEvent) {
      if (!snapshot || !(event.metaKey || event.ctrlKey)) return;
      const key = event.key.toLowerCase();
      if (key === "n") { event.preventDefault(); void createEntry(); }
      if (key === "f") { event.preventDefault(); searchRef.current?.focus(); }
      if (key === "s") { event.preventDefault(); void flush(); }
      if (event.key === "Enter") { event.preventDefault(); editorRef.current?.addSense(); }
    }
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  });

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow().onCloseRequested(async (event) => {
      if (!snapshot) return;
      event.preventDefault();
      try {
        await flush();
        await backend.closeProject();
        await getCurrentWindow().destroy();
      } catch (value) { showError(value); }
    }).then((dispose) => { unlisten = dispose; });
    return () => unlisten?.();
  }, [showError, snapshot]);

  if (!snapshot) return <><ProjectStart onProject={(project, isNew) => { setSnapshot(project); setEntries(project.entries); setError(null); setOnboarding(isNew); setSettingsOpen(isNew); }} onError={showError} />{error && <ErrorBanner error={error} className="start-error" onClose={() => setError(null)} />}</>;

  return (
    <main className="app-shell">
      <header className="app-header">
        <div className="project-title"><strong>bkuw</strong><span aria-hidden="true">/</span><span>{snapshot.project.name}</span></div>
        <div className="header-actions"><LocaleSelect />{snapshot.entrySortSettings.mode === "manual" && <Button size="small" onClick={() => setSortOrderOpen(true)}><ListOrdered size={15} />{t("sorting.manageManual")}</Button>}<Button size="small" onClick={() => setExportOpen(true)}><FileOutput size={15} />{t("export.title")}</Button><Button size="small" onClick={() => { setOnboarding(false); setSettingsOpen(true); }}><Settings size={15} />{t("common.settings")}</Button><Button size="small" variant="ghost" onClick={() => void closeProject()}><FolderX size={15} />{t("workspace.closeProject")}</Button></div>
      </header>
      {error && <ErrorBanner error={error} onClose={() => setError(null)} />}
      <Group orientation="horizontal" className="workspace">
        <Panel defaultSize="31%" minSize="240px" maxSize="520px" className="list-pane">
          <div className="list-toolbar"><div className="search-field"><Search size={16} aria-hidden="true" /><input ref={searchRef} value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t("workspace.search")} aria-label={t("workspace.search")} /></div><Button size="icon" variant="primary" onClick={() => void createEntry()} aria-label={t("workspace.newEntry")}><Plus size={17} /></Button></div>
          <EntryList entries={entries} writingSystems={snapshot.writingSystems} selectedId={entry?.id ?? null} hasQuery={Boolean(search)} onSelect={(id) => void selectEntry(id)} />
        </Panel>
        <Separator className="resize-handle" />
        <Panel minSize="440px" className="editor-pane">
          {loadingEntry ? <div className="editor-empty">{t("common.loading")}</div> : entry ? <EntryEditor key={entry.id} ref={editorRef} entry={entry} writingSystems={snapshot.writingSystems} partOfSpeechOptions={snapshot.partOfSpeechOptions} semanticDomainOptions={snapshot.semanticDomainOptions} entryOptions={entries} entrySortSettings={snapshot.entrySortSettings} onSave={saveEntry} onDelete={deleteEntry} onNavigate={(id) => void selectEntry(id)} /> : <div className="editor-empty">{t("workspace.selectEntry")}</div>}
        </Panel>
      </Group>
      {deletedId && <div className="undo-toast" role="status"><span>{t("workspace.deleted")}</span><Button size="small" variant="ghost" onClick={() => void undoDelete()}>{t("common.undo")}</Button></div>}
      <SettingsDialog open={settingsOpen} onboarding={onboarding} snapshot={snapshot} onOpenChange={(open) => { setSettingsOpen(open); if (!open) setOnboarding(false); }} onSave={saveSettings} onManageManual={() => setSortOrderOpen(true)} />
      <SortOrderDialog open={sortOrderOpen} snapshot={snapshot} onOpenChange={setSortOrderOpen} onSave={saveManualSortLayout} />
      <ExportDialog open={exportOpen} snapshot={snapshot} onOpenChange={setExportOpen} onFlush={flush} onSetAnalysisLanguage={setAnalysisLanguage} onNavigateEntry={(id) => { setExportOpen(false); void selectEntry(id); }} />
    </main>
  );
}

export default App;
