import * as Dialog from "@radix-ui/react-dialog";
import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { FileSpreadsheet, FolderOpen, Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { FontManagerButton, type FontStatus } from "../fonts/FontManagerButton";
import { backend, CommandError } from "../../lib/tauri";
import type { ProjectSnapshot } from "../../types/domain";
import { LocaleSelect } from "./LocaleSelect";
import { CsvImportWizard } from "./CsvImportWizard";

interface ProjectStartProps {
  fontStatus?: FontStatus;
  onManageFonts?(): void;
  onProject(snapshot: ProjectSnapshot, isNew: boolean): void;
  onError(error: unknown): void;
}

export function ProjectStart({ fontStatus, onManageFonts, onProject, onError }: ProjectStartProps) {
  const { t } = useTranslation();
  const canAnimateBrand = typeof window !== "undefined" && typeof window.matchMedia === "function";
  const [brandAnimationDone, setBrandAnimationDone] = useState(!canAnimateBrand);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [parentDir, setParentDir] = useState("");
  const [name, setName] = useState("");
  const [languageName, setLanguageName] = useState("");
  const [languageCode, setLanguageCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [duplicateOpen, setDuplicateOpen] = useState(false);
  const [csvImportOpen, setCsvImportOpen] = useState(false);

  useEffect(() => {
    if (!canAnimateBrand || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setBrandAnimationDone(true);
      return;
    }
    const timer = window.setTimeout(() => setBrandAnimationDone(true), 1_500);
    return () => window.clearTimeout(timer);
  }, [canAnimateBrand]);

  async function chooseParent() {
    const folder = await backend.chooseFolder();
    if (folder) setParentDir(folder);
  }

  async function createProject(event: React.FormEvent) {
    event.preventDefault();
    if (!parentDir || !name.trim()) return;
    setBusy(true);
    try {
      onProject(
        await backend.createProject({
          parentDir,
          name: name.trim(),
          languageName: languageName.trim() || null,
          languageCode: languageCode.trim() || null,
        }), true,
      );
      setDialogOpen(false);
    } catch (error) {
      if (error instanceof CommandError && error.code === "project_exists") setDuplicateOpen(true);
      else onError(error);
    } finally {
      setBusy(false);
    }
  }

  async function openProject() {
    try {
      const folder = await backend.chooseFolder();
      if (folder) onProject(await backend.openProject(folder), false);
    } catch (error) {
      onError(error);
    }
  }

  if (csvImportOpen) return <CsvImportWizard fontStatus={fontStatus} onManageFonts={onManageFonts} onCancel={() => setCsvImportOpen(false)} onProject={(snapshot) => onProject(snapshot, false)} onError={onError} />;

  return (
    <main className="start-screen">
      <header className="start-header"><strong>bkuw</strong><div className="header-preferences"><LocaleSelect />{fontStatus && onManageFonts && <FontManagerButton status={fontStatus} onClick={onManageFonts} />}</div></header>
      <section className="start-content" aria-labelledby="start-title">
        <div className="brand-mark" role="img" aria-label="bkuw">
          {Array.from("bkuw").map((letter, index) => <span className="brand-letter" data-letter-index={index} key={`${letter}-${index}`}>{letter}</span>)}
        </div>
        {brandAnimationDone && <>
          <h1 className="start-reveal start-title-reveal" id="start-title">{t("start.title")}</h1>
          <div className="start-actions start-reveal start-actions-reveal">
            <Button variant="primary" onClick={() => setDialogOpen(true)}><Plus size={17} /> {t("start.createProject")}</Button>
            <Button onClick={() => setCsvImportOpen(true)}><FileSpreadsheet size={17} /> {t("start.importCsv")}</Button>
            <Button onClick={() => void openProject()}><FolderOpen size={17} /> {t("start.openProject")}</Button>
          </div>
        </>}
      </section>
      <Dialog.Root open={dialogOpen} onOpenChange={setDialogOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="dialog-overlay" />
          <Dialog.Content className="dialog-content narrow">
            <div className="dialog-heading">
              <Dialog.Title>{t("start.createProject")}</Dialog.Title>
              <Dialog.Close asChild><Button size="icon" variant="ghost" aria-label={t("common.close")}><X size={17} /></Button></Dialog.Close>
            </div>
            <form className="stack" onSubmit={createProject}>
              <label className="field"><span>{t("start.parentFolder")}</span><div className="inline-field"><input value={parentDir} readOnly required /><Button type="button" onClick={() => void chooseParent()}>{t("start.chooseFolder")}</Button></div></label>
              <label className="field"><span>{t("start.projectName")}</span><input value={name} onChange={(event) => setName(event.target.value)} autoFocus required /></label>
              <label className="field"><span>{t("start.languageName")}</span><input value={languageName} onChange={(event) => setLanguageName(event.target.value)} /></label>
              <label className="field"><span>{t("start.languageCode")}</span><input value={languageCode} maxLength={3} pattern="[a-z]{3}" placeholder="yue" onChange={(event) => setLanguageCode(event.target.value.toLowerCase().replace(/[^a-z]/g, ""))} /><small>{t("start.languageCodeHelp")} <button className="inline-link" type="button" onClick={() => void backend.openLanguageCodeRegistry().catch(onError)}>{t("start.lookupLanguageCode")}</button></small></label>
              <div className="dialog-actions"><Button type="button" onClick={() => setDialogOpen(false)}>{t("common.cancel")}</Button><Button variant="primary" type="submit" disabled={busy || !parentDir || !name.trim()}>{busy ? t("common.loading") : t("common.create")}</Button></div>
            </form>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
      <AlertDialog.Root open={duplicateOpen} onOpenChange={setDuplicateOpen}>
        <AlertDialog.Portal>
          <AlertDialog.Overlay className="dialog-overlay" />
          <AlertDialog.Content className="dialog-content narrow">
            <AlertDialog.Title>{t("start.projectExistsTitle")}</AlertDialog.Title>
            <AlertDialog.Description>{t("start.projectExistsBody")}</AlertDialog.Description>
            <div className="dialog-actions"><AlertDialog.Cancel asChild><Button>{t("common.close")}</Button></AlertDialog.Cancel></div>
          </AlertDialog.Content>
        </AlertDialog.Portal>
      </AlertDialog.Root>
    </main>
  );
}
