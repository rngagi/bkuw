import * as Dialog from "@radix-ui/react-dialog";
import { ArrowDown, ArrowUp, Plus, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend } from "../../lib/tauri";
import { createId, type ProjectSnapshot, type WritingSystem } from "../../types/domain";

interface SettingsRequest {
  name: string;
  languageName: string | null;
  languageCode: string | null;
  description: string | null;
  writingSystems: WritingSystem[];
  partOfSpeechOptions: string[];
  semanticDomainOptions: string[];
}

interface Props {
  open: boolean;
  onboarding?: boolean;
  snapshot: ProjectSnapshot;
  onOpenChange(open: boolean): void;
  onSave(request: SettingsRequest): Promise<void>;
}

const types = ["orthography", "romanization", "transliteration", "phonemic", "phonetic", "other"] as const;

function MetadataOptionsEditor({ label, values, onChange }: {
  label: string;
  values: string[];
  onChange(values: string[]): void;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");

  function add() {
    const value = draft.trim();
    if (!value || values.includes(value)) return;
    onChange([...values, value]);
    setDraft("");
  }

  return (
    <div className="metadata-options">
      <strong>{label}</strong>
      <div className="option-chips">
        {values.map((value, index) => <span className="option-chip" key={`${value}-${index}`}>{value}<button type="button" aria-label={`${t("common.remove")} ${value}`} onClick={() => onChange(values.filter((_, itemIndex) => itemIndex !== index))}>×</button></span>)}
      </div>
      <div className="inline-field">
        <input aria-label={label} value={draft} placeholder={t("settings.optionPlaceholder")} onChange={(event) => setDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); add(); } }} />
        <Button type="button" size="small" onClick={add} disabled={!draft.trim()}>{t("settings.addOption")}</Button>
      </div>
    </div>
  );
}

export function SettingsDialog({ open, onboarding = false, snapshot, onOpenChange, onSave }: Props) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [languageName, setLanguageName] = useState("");
  const [languageCode, setLanguageCode] = useState("");
  const [description, setDescription] = useState("");
  const [systems, setSystems] = useState<WritingSystem[]>([]);
  const [partOfSpeechOptions, setPartOfSpeechOptions] = useState<string[]>([]);
  const [semanticDomainOptions, setSemanticDomainOptions] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setName(snapshot.project.name);
    setLanguageName(snapshot.project.languageName ?? "");
    setLanguageCode(snapshot.project.languageCode ?? "");
    setDescription(snapshot.project.description ?? "");
    setSystems(snapshot.writingSystems);
    setPartOfSpeechOptions(snapshot.partOfSpeechOptions);
    setSemanticDomainOptions(snapshot.semanticDomainOptions);
    setError("");
  }, [open, snapshot]);

  function patch(index: number, value: Partial<WritingSystem>) {
    setSystems((current) => current.map((item, itemIndex) => itemIndex === index ? { ...item, ...value } : item));
  }

  function setRole(index: number, role: "primary" | "secondary" | null) {
    setSystems((current) => current.map((item, itemIndex) => ({
      ...item,
      displayRole: itemIndex === index ? role : item.displayRole === role ? null : item.displayRole,
    })));
  }

  function move(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= systems.length) return;
    setSystems((current) => {
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next.map((item, order) => ({ ...item, sortOrder: order }));
    });
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!name.trim() || systems.some((item) => !item.name.trim()) || systems.filter((item) => item.displayRole === "primary").length !== 1) {
      setError(t("error.validation"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      await onSave({
        name: name.trim(), languageName: languageName.trim() || null,
        languageCode: languageCode.trim() || null, description: description.trim() || null,
        writingSystems: systems.map((item, index) => ({ ...item, name: item.name.trim(), sortOrder: index })),
        partOfSpeechOptions,
        semanticDomainOptions,
      });
      onOpenChange(false);
    } catch {
      setError(t("error.generic"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content settings-dialog">
          <div className="dialog-heading"><div><Dialog.Title>{t(onboarding ? "settings.onboardingTitle" : "settings.title")}</Dialog.Title><Dialog.Description>{t(onboarding ? "settings.onboardingBody" : "settings.writingSystemsHelp")}</Dialog.Description></div><Dialog.Close asChild><Button size="icon" variant="ghost" aria-label={t("common.close")}><X size={17} /></Button></Dialog.Close></div>
          <form onSubmit={submit} className="settings-form">
            {error && <p className="error-banner" role="alert">{error}</p>}
            <div className="settings-grid">
              <label className="field"><span>{t("start.projectName")}</span><input value={name} onChange={(event) => setName(event.target.value)} required /></label>
              <label className="field"><span>{t("start.languageName")}</span><input value={languageName} onChange={(event) => setLanguageName(event.target.value)} /></label>
              <label className="field"><span>{t("start.languageCode")}</span><input value={languageCode} maxLength={3} pattern="[a-z]{3}" placeholder="yue" onChange={(event) => setLanguageCode(event.target.value.toLowerCase().replace(/[^a-z]/g, ""))} /><small>{t("start.languageCodeHelp")} <button className="inline-link" type="button" onClick={() => void backend.openLanguageCodeRegistry()}>{t("start.lookupLanguageCode")}</button></small></label>
              <label className="field full"><span>{t("settings.description")}</span><textarea value={description} onChange={(event) => setDescription(event.target.value)} /></label>
            </div>

            <div><div className="section-heading"><div><h3>{t("settings.writingSystems")}</h3><p className="section-help">{t("settings.writingSystemsHelp")}</p></div><Button type="button" size="small" onClick={() => setSystems((current) => [...current, { id: createId(), name: "", type: "orthography", scriptCode: null, languageTag: null, displayRole: null, sortOrder: current.length, fontFamily: null, notes: null }])}><Plus size={15} />{t("settings.addWritingSystem")}</Button></div>
              <div className="writing-systems-table">
                {systems.map((system, index) => (
                  <section className="writing-system-card" key={system.id}>
                    <div className="writing-system-card-heading"><strong>{t("settings.basicFields")} {index + 1}</strong><div className="row-actions"><Button type="button" size="icon" variant="ghost" onClick={() => move(index, -1)} disabled={index === 0} aria-label={t("common.moveUp")}><ArrowUp size={15} /></Button><Button type="button" size="icon" variant="ghost" onClick={() => move(index, 1)} disabled={index === systems.length - 1} aria-label={t("common.moveDown")}><ArrowDown size={15} /></Button><Button type="button" size="icon" variant="danger" onClick={() => setSystems((current) => current.filter((_, itemIndex) => itemIndex !== index))} disabled={systems.length === 1} aria-label={t("common.remove")}><Trash2 size={15} /></Button></div></div>
                    <div className="writing-system-basics">
                      <label className="field"><span>{t("settings.name")}</span><input aria-label={t("settings.name")} value={system.name} onChange={(event) => patch(index, { name: event.target.value })} required /></label>
                      <label className="field"><span>{t("settings.type")}</span><select aria-label={t("settings.type")} value={system.type} onChange={(event) => patch(index, { type: event.target.value as WritingSystem["type"] })}>{types.map((type) => <option key={type} value={type}>{t(`settings.type_${type}`)}</option>)}</select></label>
                      <label className="field"><span>{t("settings.role")}</span><select aria-label={t("settings.role")} value={system.displayRole ?? ""} onChange={(event) => setRole(index, (event.target.value || null) as WritingSystem["displayRole"])}><option value="">{t("common.none")}</option><option value="primary">{t("settings.primary")}</option><option value="secondary">{t("settings.secondary")}</option></select></label>
                    </div>
                    <details className="writing-system-advanced"><summary>{t("settings.advancedFields")}</summary><div className="settings-grid">
                      <label className="field"><span>{t("settings.scriptCode")}</span><input aria-label={t("settings.scriptCode")} value={system.scriptCode ?? ""} onChange={(event) => patch(index, { scriptCode: event.target.value || null })} /><small>{t("settings.scriptCodeHelp")}</small></label>
                      <label className="field"><span>{t("settings.languageTag")}</span><input aria-label={t("settings.languageTag")} value={system.languageTag ?? ""} onChange={(event) => patch(index, { languageTag: event.target.value || null })} /><small>{t("settings.languageTagHelp")}</small></label>
                      <label className="field"><span>{t("settings.fontFamily")}</span><input aria-label={t("settings.fontFamily")} value={system.fontFamily ?? ""} onChange={(event) => patch(index, { fontFamily: event.target.value || null })} /><small>{t("settings.fontFamilyHelp")}</small></label>
                    </div></details>
                  </section>
                ))}
              </div>
            </div>

            <section className="metadata-settings"><h3>{t("settings.metadataTitle")}</h3><p className="section-help">{t("settings.metadataHelp")}</p><div className="two-columns"><MetadataOptionsEditor label={t("settings.partOfSpeechOptions")} values={partOfSpeechOptions} onChange={setPartOfSpeechOptions} /><MetadataOptionsEditor label={t("settings.semanticDomainOptions")} values={semanticDomainOptions} onChange={setSemanticDomainOptions} /></div></section>
            <div className="dialog-actions"><Button type="button" onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button><Button type="submit" variant="primary" disabled={busy}>{busy ? t("common.loading") : t("common.save")}</Button></div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
