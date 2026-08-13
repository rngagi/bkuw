import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { ArrowDown, ArrowRight, ArrowUp, Plus, Save, Trash2 } from "lucide-react";
import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";
import {
  useFieldArray,
  useForm,
  useWatch,
  type Control,
  type UseFormRegister,
  type UseFormRegisterReturn,
  type UseFormSetValue,
} from "react-hook-form";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { CommandError } from "../../lib/tauri";
import { displayWritingSystemText } from "../../lib/writingSystems";
import { createId, type EntrySortSettings, type EntrySummary, type LexicalEntry, type Sense, type WritingSystem } from "../../types/domain";

export interface EntryEditorHandle {
  flush(): Promise<LexicalEntry | undefined>;
  addSense(): void;
}

interface Props {
  entry: LexicalEntry;
  writingSystems: WritingSystem[];
  partOfSpeechOptions: string[];
  semanticDomainOptions: string[];
  entryOptions: EntrySummary[];
  entrySortSettings?: EntrySortSettings;
  onSave(entry: LexicalEntry): Promise<LexicalEntry>;
  onDelete(): Promise<void>;
  onNavigate(id: string): void;
}

type Register = UseFormRegister<LexicalEntry>;
type SetValue = UseFormSetValue<LexicalEntry>;

function normalizeAggregateOrder(entry: LexicalEntry): LexicalEntry {
  return {
    ...entry,
    forms: entry.forms.map((form, sortOrder) => ({ ...form, sortOrder })),
    senses: entry.senses.map((sense, sortOrder) => ({
      ...sense,
      sortOrder,
      examples: sense.examples.map((example, exampleSortOrder) => ({
        ...example,
        sortOrder: exampleSortOrder,
        forms: example.forms.map((form, formSortOrder) => ({ ...form, sortOrder: formSortOrder })),
      })),
    })),
    relations: entry.relations.map((relation, sortOrder) => ({ ...relation, sortOrder })),
  };
}

function orderButtons(move: (from: number, to: number) => void, index: number, count: number) {
  return { moveUp: () => move(index, index - 1), moveDown: () => move(index, index + 1), first: index === 0, last: index === count - 1 };
}

function WritingSystemInput({ system, registration, autoFocus = false }: {
  system?: WritingSystem;
  registration: UseFormRegisterReturn;
  autoFocus?: boolean;
}) {
  const { t } = useTranslation();
  const delimiters = system?.type === "phonemic" ? ["/", "/"] : system?.type === "phonetic" ? ["[", "]"] : null;
  const input = <input autoFocus={autoFocus} aria-label={t("entry.text")} style={{ fontFamily: system?.fontFamily ?? undefined }} {...registration} />;
  return delimiters ? <div className="transcription-input"><span aria-hidden="true">{delimiters[0]}</span>{input}<span aria-hidden="true">{delimiters[1]}</span></div> : input;
}

function ExampleEditor({ control, register, senseIndex, exampleIndex, writingSystems, onRemove, onMove, count }: {
  control: Control<LexicalEntry>; register: Register; senseIndex: number; exampleIndex: number;
  writingSystems: WritingSystem[]; onRemove(): void; onMove(from: number, to: number): void; count: number;
}) {
  const { t } = useTranslation();
  const name = `senses.${senseIndex}.examples.${exampleIndex}.forms` as const;
  const forms = useFieldArray({ control, name });
  const watchedForms = useWatch({ control, name }) ?? [];
  const used = new Set(watchedForms.map((form) => form.writingSystemId));
  const nextSystem = writingSystems.find((system) => !used.has(system.id));
  const order = orderButtons(onMove, exampleIndex, count);
  return (
    <div className="nested-block example-block">
      <div className="nested-heading"><h4>{t("entry.example", { number: exampleIndex + 1 })}</h4><div className="row-actions"><Button type="button" size="icon" variant="ghost" disabled={order.first} onClick={order.moveUp} aria-label={t("common.moveUp")}><ArrowUp size={14} /></Button><Button type="button" size="icon" variant="ghost" disabled={order.last} onClick={order.moveDown} aria-label={t("common.moveDown")}><ArrowDown size={14} /></Button><Button type="button" size="icon" variant="danger" onClick={onRemove} aria-label={t("common.remove")}><Trash2 size={14} /></Button></div></div>
      {forms.fields.map((field, formIndex) => {
        const systemId = watchedForms[formIndex]?.writingSystemId ?? field.writingSystemId;
        const system = writingSystems.find((item) => item.id === systemId);
        return <div className="form-row" key={field.id}><span className="form-system-label">{system?.name ?? t("entry.writingSystem")}</span><WritingSystemInput system={system} registration={register(`senses.${senseIndex}.examples.${exampleIndex}.forms.${formIndex}.text` as const)} /><Button type="button" size="icon" variant="ghost" onClick={() => forms.remove(formIndex)} aria-label={t("common.remove")}><Trash2 size={14} /></Button></div>;
      })}
      <Button type="button" size="small" variant="ghost" disabled={!nextSystem} onClick={() => { if (nextSystem) forms.append({ id: createId(), writingSystemId: nextSystem.id, text: "", sortOrder: forms.fields.length }); }}><Plus size={14} />{t("entry.addExampleForm")}</Button>
      <div className="two-columns"><label className="field"><span>{t("entry.translation")}</span><input {...register(`senses.${senseIndex}.examples.${exampleIndex}.translation` as const)} /></label><label className="field"><span>{t("entry.exampleNotes")}</span><input {...register(`senses.${senseIndex}.examples.${exampleIndex}.notes` as const)} /></label></div>
    </div>
  );
}

function metadataChoices(configured: string[], current?: string | null) {
  return current && !configured.includes(current) ? [current, ...configured] : configured;
}

function SenseEditor({ control, register, sense, index, writingSystems, partOfSpeechOptions, semanticDomainOptions, onRemove, onMove, count }: {
  control: Control<LexicalEntry>; register: Register; sense: Sense & { id: string }; index: number;
  writingSystems: WritingSystem[]; partOfSpeechOptions: string[]; semanticDomainOptions: string[];
  onRemove(): void; onMove(from: number, to: number): void; count: number;
}) {
  const { t } = useTranslation();
  const examples = useFieldArray({ control, name: `senses.${index}.examples` as const });
  const order = orderButtons(onMove, index, count);
  const primary = writingSystems.find((system) => system.displayRole === "primary") ?? writingSystems[0];
  return (
    <section className="editor-section sense-section">
      <div className="section-heading"><h3>{t("entry.sense", { number: index + 1 })}</h3><div className="row-actions"><Button type="button" size="icon" variant="ghost" disabled={order.first} onClick={order.moveUp} aria-label={t("common.moveUp")}><ArrowUp size={15} /></Button><Button type="button" size="icon" variant="ghost" disabled={order.last} onClick={order.moveDown} aria-label={t("common.moveDown")}><ArrowDown size={15} /></Button><Button type="button" size="icon" variant="danger" onClick={onRemove} aria-label={t("common.remove")}><Trash2 size={15} /></Button></div></div>
      <div className="two-columns"><label className="field"><span>{t("entry.gloss")}</span><input {...register(`senses.${index}.gloss` as const)} /></label><label className="field"><span>{t("entry.partOfSpeech")}</span><select {...register(`senses.${index}.partOfSpeech` as const, { setValueAs: (value) => value || null })}><option value="">{t("common.none")}</option>{metadataChoices(partOfSpeechOptions, sense.partOfSpeech).map((value) => <option key={value} value={value}>{value}</option>)}</select></label></div>
      <label className="field"><span>{t("entry.definition")}</span><textarea {...register(`senses.${index}.definition` as const)} /></label>
      <label className="field"><span>{t("entry.semanticDomain")}</span><select {...register(`senses.${index}.semanticDomain` as const, { setValueAs: (value) => value || null })}><option value="">{t("common.none")}</option>{metadataChoices(semanticDomainOptions, sense.semanticDomain).map((value) => <option key={value} value={value}>{value}</option>)}</select></label>
      <div className="subsection-heading"><h4>{t("entry.examples")}</h4><Button type="button" size="small" onClick={() => examples.append({ id: createId(), translation: null, notes: null, sortOrder: examples.fields.length, forms: primary ? [{ id: createId(), writingSystemId: primary.id, text: "", sortOrder: 0 }] : [] })}><Plus size={14} />{t("entry.addExample")}</Button></div>
      {examples.fields.map((example, exampleIndex) => <ExampleEditor key={example.id} control={control} register={register} senseIndex={index} exampleIndex={exampleIndex} writingSystems={writingSystems} onRemove={() => examples.remove(exampleIndex)} onMove={examples.move} count={examples.fields.length} />)}
    </section>
  );
}

function RelationEditor({ control, register, setValue, index, entryId, entryOptions, primaryWritingSystem, onNavigate, onRemove }: {
  control: Control<LexicalEntry>; register: Register; setValue: SetValue; index: number; entryId: string;
  entryOptions: EntrySummary[]; primaryWritingSystem?: WritingSystem; onNavigate(id: string): void; onRemove(): void;
}) {
  const { t } = useTranslation();
  const targetEntryId = useWatch({ control, name: `relations.${index}.targetEntryId` });
  const choices = entryOptions.filter((option) => option.id !== entryId);
  const labelFor = (option: EntrySummary) => `${option.primaryForm ? displayWritingSystemText(option.primaryForm, primaryWritingSystem) : t("workspace.untitled")} — ${option.id.slice(0, 8)}`;
  const selected = choices.find((option) => option.id === targetEntryId);
  const [query, setQuery] = useState(selected ? labelFor(selected) : "");
  useEffect(() => { if (selected) setQuery(labelFor(selected)); }, [selected?.id, selected?.primaryForm]);
  return (
    <div className="relation-row">
      <select aria-label={t("entry.relationType")} {...register(`relations.${index}.relationType`)}><option value="root">{t("entry.root")}</option><option value="base">{t("entry.base")}</option></select>
      <div><input aria-label={t("entry.linkedEntry")} list={`entry-options-${index}`} value={query} onChange={(event) => { const nextQuery = event.target.value; const match = choices.find((option) => labelFor(option) === nextQuery); setQuery(nextQuery); setValue(`relations.${index}.targetEntryId`, match?.id ?? null, { shouldDirty: true }); }} /><datalist id={`entry-options-${index}`}>{choices.map((option) => <option key={option.id} value={labelFor(option)} />)}</datalist></div>
      <input aria-label={t("entry.fallbackText")} placeholder={t("entry.fallbackText")} {...register(`relations.${index}.fallbackText`)} />
      {targetEntryId && <Button type="button" size="icon" variant="ghost" onClick={() => onNavigate(targetEntryId)} aria-label={t("entry.navigate")}><ArrowRight size={15} /></Button>}
      <Button type="button" size="icon" variant="ghost" onClick={onRemove} aria-label={t("common.remove")}><Trash2 size={14} /></Button>
    </div>
  );
}

export const EntryEditor = forwardRef<EntryEditorHandle, Props>(function EntryEditor({ entry, writingSystems, partOfSpeechOptions, semanticDomainOptions, entryOptions, entrySortSettings, onSave, onDelete, onNavigate }, ref) {
  const { t } = useTranslation();
  const sortSettings = entrySortSettings ?? { version: 1 as const, mode: "auto" as const, writingSystemId: writingSystems[0]?.id ?? "", alphabet: [] };
  const { control, register, reset, getValues, setValue, watch } = useForm<LexicalEntry>({ defaultValues: entry });
  const senses = useFieldArray({ control, name: "senses" });
  const relations = useFieldArray({ control, name: "relations" });
  const watchedForms = useWatch({ control, name: "forms" }) ?? [];
  const sectionOverride = useWatch({ control, name: "sectionOverride" });
  const [status, setStatus] = useState<"saved" | "dirty" | "saving" | "error">("saved");
  const [saveFailure, setSaveFailure] = useState<{ code: string; message: string; details?: string } | null>(null);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [pendingSection, setPendingSection] = useState<string | null | undefined>(undefined);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const composing = useRef(false);
  const dirty = useRef(false);
  const applyingSavedValue = useRef(false);
  const pending = useRef(false);
  const inFlight = useRef<Promise<LexicalEntry | undefined> | null>(null);
  const mountedEntryId = useRef(entry.id);
  const latestCommitted = useRef(entry);

  useEffect(() => {
    if (mountedEntryId.current === entry.id && (composing.current || inFlight.current)) return;
    mountedEntryId.current = entry.id;
    latestCommitted.current = entry;
    applyingSavedValue.current = true;
    reset(entry);
    applyingSavedValue.current = false;
    dirty.current = false;
    setStatus("saved");
    setSaveFailure(null);
  }, [entry, reset]);

  const saveNowRef = useRef<(propagateError?: boolean) => Promise<LexicalEntry | undefined>>(async () => undefined);
  const saveNow = useCallback(async (propagateError = false): Promise<LexicalEntry | undefined> => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    if (inFlight.current) {
      pending.current = true;
      const saved = await inFlight.current;
      if (propagateError) return (await saveNowRef.current(true)) ?? saved;
      return saved;
    }
    if (!dirty.current && status !== "error") return latestCommitted.current;

    const operation = (async (): Promise<LexicalEntry | undefined> => {
      setStatus("saving");
      setSaveFailure(null);
      const draft = normalizeAggregateOrder(getValues());
      const serialized = JSON.stringify(draft);
      try {
        const saved = await onSave(draft);
        latestCommitted.current = saved;
        if (mountedEntryId.current !== saved.id) return saved;
        const current = getValues();
        if (JSON.stringify(normalizeAggregateOrder(current)) === serialized && !composing.current) {
          applyingSavedValue.current = true;
          setValue("revision", saved.revision, { shouldDirty: false });
          setValue("updatedAt", saved.updatedAt, { shouldDirty: false });
          applyingSavedValue.current = false;
          dirty.current = false;
          setStatus("saved");
        } else {
          setValue("revision", saved.revision, { shouldDirty: false });
          setValue("updatedAt", saved.updatedAt, { shouldDirty: false });
          dirty.current = true;
          setStatus("dirty");
          pending.current = !composing.current;
        }
        return saved;
      } catch (error) {
        const failure = error instanceof CommandError
          ? { code: error.code, message: error.message, details: error.details }
          : { code: "generic", message: error instanceof Error ? error.message : String(error) };
        setSaveFailure(failure);
        dirty.current = true;
        setStatus("error");
        if (propagateError) throw error;
        return undefined;
      } finally {
        inFlight.current = null;
        if (pending.current && !composing.current) {
          pending.current = false;
          queueMicrotask(() => void saveNowRef.current(false));
        }
      }
    })();
    inFlight.current = operation;
    return operation;
  }, [getValues, onSave, setValue, status]);

  useEffect(() => { saveNowRef.current = saveNow; }, [saveNow]);
  useEffect(() => {
    const subscription = watch(() => {
      if (applyingSavedValue.current) return;
      dirty.current = true;
      setStatus("dirty");
      setSaveFailure(null);
      if (timer.current) clearTimeout(timer.current);
      if (composing.current) return;
      timer.current = setTimeout(() => void saveNowRef.current(false), 700);
    });
    return () => { subscription.unsubscribe(); if (timer.current) clearTimeout(timer.current); };
  }, [watch]);

  function addSense() {
    senses.append({ id: createId(), gloss: null, definition: null, partOfSpeech: null, semanticDomain: null, sortOrder: senses.fields.length, examples: [] });
  }

  useImperativeHandle(ref, () => ({ flush: () => saveNow(true), addSense }), [saveNow]);

  return <>
    <form className="entry-editor" onCompositionStart={() => { composing.current = true; if (timer.current) clearTimeout(timer.current); }} onCompositionEnd={() => { composing.current = false; if (timer.current) clearTimeout(timer.current); timer.current = setTimeout(() => void saveNowRef.current(false), 700); }} onSubmit={(event) => { event.preventDefault(); void saveNow(false); }}>
      <header className="editor-toolbar"><span className={`save-status ${status}`} role="status" aria-live="polite">{status === "saving" ? t("workspace.saving") : status === "saved" ? t("workspace.saved") : status === "error" ? t("workspace.saveFailed") : t("workspace.unsaved")}</span><Button type="submit" size="small"><Save size={15} />{t("common.save")}</Button><Button type="button" size="small" variant="danger" onClick={() => setDeleteConfirmOpen(true)}><Trash2 size={15} />{t("entry.deleteEntry")}</Button></header>
      {saveFailure && <div className="error-banner save-error" role="alert"><strong>{t("entry.saveErrorTitle")}</strong><span>{saveFailure.code === "generic" ? saveFailure.message : t(`error.${saveFailure.code}`, { defaultValue: saveFailure.message })}</span>{saveFailure.details && <details><summary>{t("common.details")}</summary><code>{saveFailure.details}</code></details>}</div>}
      <div className="editor-scroll">
        <section className="editor-section"><div className="section-heading"><h2>{t("entry.forms")}</h2></div>{watchedForms.map((form, index) => { const system = writingSystems.find((item) => item.id === form.writingSystemId); return <div className="form-row" key={form.id}><span className="form-system-label">{system?.name ?? t("entry.writingSystem")}</span><WritingSystemInput autoFocus={index === 0} system={system} registration={register(`forms.${index}.text`)} /><span /></div>; })}<label className="field"><span>{t("sorting.entrySection")}</span><select aria-label={t("sorting.entrySection")} value={sectionOverride ?? ""} disabled={sortSettings.mode === "manual"} onChange={(event) => setPendingSection(event.target.value || null)}><option value="">{t("sorting.automaticSection")}</option>{sortSettings.alphabet.map((item) => <option key={item} value={item.toUpperCase()}>{item.toUpperCase()}</option>)}</select><small>{t(sortSettings.mode === "manual" ? "sorting.overrideDisabled" : "sorting.entrySectionHelp")}</small></label><label className="field"><span>{t("entry.notes")}</span><textarea {...register("notes")} /></label></section>
        <div className="section-heading standalone"><h2>{t("entry.senses")}</h2><Button type="button" size="small" onClick={addSense}><Plus size={14} />{t("entry.addSense")}</Button></div>
        {senses.fields.map((sense, index) => <SenseEditor key={sense.id} control={control} register={register} sense={sense} index={index} writingSystems={writingSystems} partOfSpeechOptions={partOfSpeechOptions} semanticDomainOptions={semanticDomainOptions} onRemove={() => senses.remove(index)} onMove={senses.move} count={senses.fields.length} />)}
        <section className="editor-section"><div className="section-heading"><h2>{t("entry.relations")}</h2><Button type="button" size="small" onClick={() => relations.append({ id: createId(), targetEntryId: null, relationType: "root", fallbackText: "", notes: null, sortOrder: relations.fields.length })}><Plus size={14} />{t("entry.addRelation")}</Button></div><p className="section-help">{t("entry.fallbackHelp")}</p>{relations.fields.map((relation, index) => <RelationEditor key={relation.id} control={control} register={register} setValue={setValue} index={index} entryId={entry.id} entryOptions={entryOptions} primaryWritingSystem={writingSystems.find((system) => system.displayRole === "primary")} onNavigate={onNavigate} onRemove={() => relations.remove(index)} />)}</section>
      </div>
    </form>
    <AlertDialog.Root open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}><AlertDialog.Portal><AlertDialog.Overlay className="dialog-overlay" /><AlertDialog.Content className="dialog-content narrow"><AlertDialog.Title>{t("entry.deleteTitle")}</AlertDialog.Title><AlertDialog.Description>{t("entry.deleteBody")}</AlertDialog.Description><div className="dialog-actions"><AlertDialog.Cancel asChild><Button>{t("common.cancel")}</Button></AlertDialog.Cancel><AlertDialog.Action asChild><Button variant="danger" onClick={() => void onDelete()}>{t("entry.confirmDelete")}</Button></AlertDialog.Action></div></AlertDialog.Content></AlertDialog.Portal></AlertDialog.Root>
    <AlertDialog.Root open={pendingSection !== undefined} onOpenChange={(open) => { if (!open) setPendingSection(undefined); }}><AlertDialog.Portal><AlertDialog.Overlay className="dialog-overlay" /><AlertDialog.Content className="dialog-content narrow"><AlertDialog.Title>{t("sorting.overrideTitle")}</AlertDialog.Title><AlertDialog.Description>{t("sorting.overrideBody", { section: pendingSection ?? t("sorting.automaticSection") })}</AlertDialog.Description><div className="dialog-actions"><AlertDialog.Cancel asChild><Button>{t("common.cancel")}</Button></AlertDialog.Cancel><AlertDialog.Action asChild><Button variant="primary" onClick={() => { setValue("sectionOverride", pendingSection ?? null, { shouldDirty: true }); setPendingSection(undefined); }}>{t("sorting.confirmOverride")}</Button></AlertDialog.Action></div></AlertDialog.Content></AlertDialog.Portal></AlertDialog.Root>
  </>;
});
