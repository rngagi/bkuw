import * as Dialog from "@radix-ui/react-dialog";
import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { FolderOpen, Plus, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type { ProjectSnapshot } from "../../types/domain";
import { LocaleSelect } from "./LocaleSelect";

interface ProjectStartProps {
  onProject(snapshot: ProjectSnapshot, isNew: boolean): void;
  onError(error: unknown): void;
}

export function ProjectStart({ onProject, onError }: ProjectStartProps) {
  const { t } = useTranslation();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [parentDir, setParentDir] = useState("");
  const [name, setName] = useState("");
  const [languageName, setLanguageName] = useState("");
  const [languageCode, setLanguageCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [duplicateOpen, setDuplicateOpen] = useState(false);

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

  return (
    <main className="start-screen">
      <header className="start-header"><strong>bkuw</strong><LocaleSelect /></header>
      <section className="start-content" aria-labelledby="start-title">
        <div className="brand-mark" aria-hidden="true">b</div>
        <h1 id="start-title">{t("start.title")}</h1>
        <p>{t("start.body")}</p>
        <div className="start-actions">
          <Button variant="primary" onClick={() => setDialogOpen(true)}><Plus size={17} /> {t("start.createProject")}</Button>
          <Button onClick={() => void openProject()}><FolderOpen size={17} /> {t("start.openProject")}</Button>
        </div>
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
