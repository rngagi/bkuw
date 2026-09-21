import * as Dialog from "@radix-ui/react-dialog";
import { Check, CloudUpload, Copy, ExternalLink, Info, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { backend, CommandError } from "../../lib/tauri";
import type { ProjectSnapshot, PublishPreview, PublishProgress, PublishResult, PublishSettings, PublishState } from "../../types/domain";

interface Props {
  open: boolean;
  snapshot: ProjectSnapshot;
  onOpenChange(open: boolean): void;
  onFlush(): Promise<unknown>;
  onDeploymentChange?(published: boolean): void;
}

function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
function bytes(value: number) { return value < 1024 ? `${value} B` : value < 1024 ** 2 ? `${(value / 1024).toFixed(1)} KiB` : `${(value / 1024 ** 2).toFixed(1)} MiB`; }

export function PublishDialog({ open, snapshot, onOpenChange, onFlush, onDeploymentChange }: Props) {
  const { t } = useTranslation();
  const heading = useRef<HTMLHeadingElement>(null);
  const tokenInput = useRef<HTMLInputElement>(null);
  const [step, setStep] = useState(0);
  const [state, setState] = useState<PublishState | null>(null);
  const [settings, setSettings] = useState<PublishSettings | null>(null);
  const [accountId, setAccountId] = useState("");
  const [tokenReady, setTokenReady] = useState(false);
  const [subdomain, setSubdomain] = useState("");
  const [emailVerified, setEmailVerified] = useState(false);
  const [r2Ready, setR2Ready] = useState(false);
  const [preview, setPreview] = useState<PublishPreview | null>(null);
  const [progress, setProgress] = useState<PublishProgress | null>(null);
  const [result, setResult] = useState<PublishResult | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setStep(0); setPreview(null); setProgress(null); setResult(null); setError(null); setTokenReady(false);
    setBusy("loading");
    backend.getPublishState().then((value) => {
      setState(value); setSettings(clone(value.settings)); setAccountId(value.connection.accountId ?? value.deployment?.accountId ?? "");
      setSubdomain(value.connection.workersSubdomain ?? value.deployment?.workersSubdomain ?? value.settings.workerName);
      if (value.connection.connected) { setEmailVerified(true); setR2Ready(true); }
    }).catch(showError).finally(() => setBusy(null));
  }, [open, snapshot.project.id]);
  useEffect(() => { heading.current?.focus(); }, [step]);

  function showError(value: unknown) {
    setError(value instanceof CommandError ? t(`error.${value.code}`, { defaultValue: value.message }) : t("error.generic"));
  }
  function patch(value: Partial<PublishSettings>) { setSettings((current) => current ? { ...current, ...value } : current); setPreview(null); setResult(null); }
  async function connect() {
    setBusy("connect"); setError(null);
    try {
      const connection = await backend.connectCloudflare({ accountId, apiToken: tokenInput.current?.value ?? "", requestedSubdomain: subdomain.trim() || null });
      if (tokenInput.current) tokenInput.current.value = "";
      setTokenReady(false); setState((current) => current ? { ...current, connection } : current); setStep(2);
    } catch (value) { showError(value); } finally { setBusy(null); }
  }
  async function saveAndPreview() {
    if (!settings) return;
    setBusy("preview"); setError(null);
    try {
      await onFlush();
      const saved = await backend.savePublishSettings(settings);
      setSettings(saved); setPreview(await backend.previewPublish()); setStep(3);
    } catch (value) { showError(value); } finally { setBusy(null); }
  }
  async function publish() {
    if (!preview) return;
    setBusy("publish"); setError(null); setResult(null);
    try {
      const value = await backend.publishSite(preview.snapshotToken, setProgress);
      setResult(value); setState(await backend.getPublishState()); onDeploymentChange?.(true);
    } catch (value) { showError(value); } finally { setBusy(null); }
  }
  async function disconnect() {
    const id = state?.connection.accountId ?? accountId;
    if (!id) return;
    await backend.disconnectCloudflare(id);
    setState((current) => current ? { ...current, connection: { connected: false, accountId: null, workersSubdomain: null, credentialPersisted: false } } : current);
    setStep(1);
  }
  async function retryCleanup() {
    setBusy("cleanup"); setError(null);
    try { await backend.retryPublishCleanup(); setState(await backend.getPublishState()); }
    catch (value) { showError(value); } finally { setBusy(null); }
  }
  const primaryId = snapshot.writingSystems.find((item) => item.displayRole === "primary")?.id ?? snapshot.writingSystems[0]?.id;
  const blockers = preview?.issues.filter((item) => item.severity === "error") ?? [];
  const canCancel = progress && !["deploying", "verifying", "cleaning", "complete"].includes(progress.phase);

  return <Dialog.Root open={open} onOpenChange={(value) => { if (busy === "publish") return; onOpenChange(value); }}>
    <Dialog.Portal>
      <Dialog.Overlay className="dialog-overlay" />
      <Dialog.Content className="dialog-content publish-dialog" aria-describedby="publish-description">
        <div className="dialog-heading">
          <div><Dialog.Title ref={heading} tabIndex={-1}>{state?.deployment ? t("publish.updateTitle") : t("publish.title")}</Dialog.Title><Dialog.Description id="publish-description">{t("publish.description")}</Dialog.Description></div>
          <Dialog.Close asChild><Button size="icon" variant="ghost" aria-label={t("common.close")}><X size={17} /></Button></Dialog.Close>
        </div>
        <ol className="publish-steps" aria-label={t("publish.stepsLabel")}>
          {["start", "connect", "settings", "review"].map((key, index) => <li key={key} className={index === step ? "active" : index < step ? "done" : ""}><span>{index < step ? <Check size={13} /> : index + 1}</span>{t(`publish.step.${key}`)}</li>)}
        </ol>
        {error && <div className="error-banner" role="alert">{error}</div>}
        {busy === "loading" && <div className="publish-loading">{t("common.loading")}</div>}

        {settings && step === 0 && <section className="publish-section">
          <div className="publish-callout"><Info size={20} /><div><strong>{t("publish.publicWarning")}</strong><p>{t("publish.costWarning")}</p><p>{t("publish.manualOnly")}</p></div></div>
          <div className="publish-reference-links"><Button size="small" variant="ghost" onClick={() => void backend.openPublishHelp("workersPricing")}><ExternalLink size={14} />{t("publish.workersPricing")}</Button><Button size="small" variant="ghost" onClick={() => void backend.openPublishHelp("r2GettingStarted")}><ExternalLink size={14} />{t("publish.r2Guide")}</Button><Button size="small" variant="ghost" onClick={() => void backend.openPublishHelp("accountSetup")}><ExternalLink size={14} />{t("publish.accountGuide")}</Button></div>
          {state?.deployment && <div className="publish-existing"><strong>{t("publish.currentSite")}</strong><a href={state.deployment.publicUrl} onClick={(event) => { event.preventDefault(); void backend.openPublicWebsite(state.deployment!.publicUrl); }}>{state.deployment.publicUrl}</a><span>{t("publish.lastPublished", { date: new Date(state.deployment.lastPublishedAt).toLocaleString() })}</span>{state.deployment.cleanupPending && <Button size="small" variant="secondary" disabled={busy !== null} onClick={() => void retryCleanup()}>{busy === "cleanup" ? t("publish.cleaning") : t("publish.retryCleanup")}</Button>}</div>}
          <div className="publish-checklist">
            <h3>{t("publish.accountChecklist")}</h3>
            <Button variant="secondary" onClick={() => void backend.openPublishHelp("signup")}><ExternalLink size={15} />{t("publish.createAccount")}</Button>
            <ol><li>{t("publish.accountStep.email")}</li><li>{t("publish.accountStep.verify")}</li><li>{t("publish.accountStep.login")}</li></ol>
            <label className="check-row"><input type="checkbox" checked={emailVerified} onChange={(event) => setEmailVerified(event.target.checked)} />{t("publish.emailVerified")}</label>
            <Button variant="secondary" onClick={() => void backend.openPublishHelp("r2")}><ExternalLink size={15} />{t("publish.openR2")}</Button>
            <p className="field-help">{t("publish.r2Help")}</p>
            <label className="check-row"><input type="checkbox" checked={r2Ready} onChange={(event) => setR2Ready(event.target.checked)} />{t("publish.r2Ready")}</label>
          </div>
          <div className="dialog-actions"><Button variant="primary" disabled={!emailVerified || !r2Ready} onClick={() => setStep(state?.connection.connected ? 2 : 1)}>{t("common.next")}</Button></div>
        </section>}

        {settings && step === 1 && <section className="publish-section">
          <h3>{t("publish.connectHeading")}</h3><p>{t("publish.accountIdHelp")}</p>
          <label className="field"><span>{t("publish.accountId")}</span><input value={accountId} maxLength={32} spellCheck={false} onChange={(event) => setAccountId(event.target.value.trim())} placeholder="0123456789abcdef0123456789abcdef" /></label>
          <div className="publish-token-action"><Button variant="secondary" disabled={accountId.length !== 32} onClick={() => void backend.openCloudflareTokenPage(accountId).catch(showError)}><ExternalLink size={15} />{t("publish.createToken")}</Button><p>{t("publish.permissions")}</p></div>
          <label className="field"><span>{t("publish.apiToken")}</span><input ref={tokenInput} type="password" autoComplete="off" onChange={(event) => setTokenReady(event.target.value.length >= 20)} /></label>
          <label className="field"><span>{t("publish.subdomain")}</span><input value={subdomain} onChange={(event) => setSubdomain(event.target.value.toLowerCase())} /><small>{t("publish.subdomainHelp")}</small></label>
          <div className="dialog-actions"><Button variant="ghost" onClick={() => setStep(0)}>{t("common.back")}</Button><Button variant="primary" disabled={busy !== null || accountId.length !== 32 || !tokenReady} onClick={() => void connect()}>{busy === "connect" ? t("publish.checking") : t("publish.connect")}</Button></div>
        </section>}

        {settings && step === 2 && <section className="publish-section">
          <div className="publish-connection"><span><Check size={15} />{t("publish.connected", { account: state?.connection.accountId })}</span><Button size="small" variant="ghost" onClick={() => void disconnect()}>{t("publish.disconnect")}</Button></div>
          {!state?.connection.credentialPersisted && <p className="warning-banner">{t("publish.sessionCredential")}</p>}
          <div className="publish-form-grid">
            <label className="field"><span>{t("publish.siteTitle")}</span><input required value={settings.title} onChange={(event) => patch({ title: event.target.value })} /></label>
            <label className="field"><span>{t("publish.locale")}</span><select value={settings.locale} onChange={(event) => patch({ locale: event.target.value as "zh-TW" | "en" })}><option value="zh-TW">繁體中文（台灣）</option><option value="en">English</option></select></label>
            <label className="field full"><span>{t("publish.metaDescription")}</span><input value={settings.description ?? ""} onChange={(event) => patch({ description: event.target.value || null })} /></label>
            <label className="field full"><span>{t("publish.infoMarkdown")}</span><textarea rows={5} value={settings.infoMarkdown ?? ""} onChange={(event) => patch({ infoMarkdown: event.target.value || null })} /><small>{t("publish.infoHelp")}</small></label>
          </div>
          <fieldset className="publish-fieldset"><legend>{t("publish.publicFields")}</legend>
            <label className="check-row"><input type="checkbox" checked={settings.includeEntryNotes} onChange={(event) => patch({ includeEntryNotes: event.target.checked })} />{t("publish.entryNotes")}</label>
            <label className="check-row"><input type="checkbox" checked={settings.includeExampleNotes} onChange={(event) => patch({ includeExampleNotes: event.target.checked })} />{t("publish.exampleNotes")}</label>
            <label className="check-row"><input type="checkbox" checked={settings.includeRelations} onChange={(event) => patch({ includeRelations: event.target.checked })} />{t("publish.relations")}</label>
          </fieldset>
          <fieldset className="publish-fieldset"><legend>{t("publish.writingSystems")}</legend>{snapshot.writingSystems.map((system) => {
            const primary = system.id === primaryId, checked = settings.writingSystemIds.includes(system.id);
            return <label className="check-row" key={system.id}><input type="checkbox" checked={checked} disabled={primary} onChange={(event) => patch({ writingSystemIds: event.target.checked ? [...settings.writingSystemIds, system.id] : settings.writingSystemIds.filter((id) => id !== system.id) })} />{system.name}{primary && <small>{t("publish.primaryRequired")}</small>}</label>;
          })}</fieldset>
          <div className="publish-resource-grid">
            <label className="field"><span>Worker</span><input value={settings.workerName} disabled={Boolean(state?.deployment)} onChange={(event) => patch({ workerName: event.target.value.toLowerCase() })} /></label>
            <label className="field"><span>R2 bucket</span><input value={settings.bucketName} disabled={Boolean(state?.deployment)} onChange={(event) => patch({ bucketName: event.target.value.toLowerCase() })} /></label>
          </div>
          <div className="dialog-actions"><Button variant="ghost" onClick={() => setStep(0)}>{t("common.back")}</Button><Button variant="primary" disabled={busy !== null || !settings.title.trim()} onClick={() => void saveAndPreview()}>{busy === "preview" ? t("publish.checking") : t("publish.check")}</Button></div>
        </section>}

        {settings && step === 3 && preview && <section className="publish-section">
          <div className="publish-counts"><span>{t("publish.count.entries", { count: preview.entryCount })}</span><span>{t("publish.count.senses", { count: preview.senseCount })}</span><span>{t("publish.count.examples", { count: preview.exampleCount })}</span><span>{t("publish.count.images", { count: preview.imageCount })}</span><span>{t("publish.count.audio", { count: preview.audioCount })}</span></div>
          <div className="publish-transfer"><strong>{t("publish.transferTitle")}</strong><span>{t("publish.transfer", { upload: preview.uploadMediaCount, keep: preview.unchangedMediaCount, remove: preview.deleteMediaCount, bytes: bytes(preview.uploadBytes) })}</span>{preview.publicUrl && <code>{preview.publicUrl}</code>}</div>
          {preview.issues.length > 0 && <div className="publish-issues">{preview.issues.map((issue, index) => <p className={issue.severity} key={`${issue.code}-${index}`}>{t(issue.code, { defaultValue: issue.code })}{issue.details ? `: ${issue.details}` : ""}</p>)}</div>}
          {progress && <div className="publish-progress" role="status"><div><span>{t(`publish.phase.${progress.phase}`)}</span><span>{progress.totalItems ? `${progress.completedItems}/${progress.totalItems}` : ""}</span></div><progress max={Math.max(progress.totalItems, 1)} value={progress.completedItems} /></div>}
          {result && <div className="publish-success"><CloudUpload size={28} /><div><strong>{t("publish.complete")}</strong><a href={result.publicUrl} onClick={(event) => { event.preventDefault(); void backend.openPublicWebsite(result.publicUrl); }}>{result.publicUrl}</a><p>{t("publish.resultStats", { upload: result.uploadedMediaCount, keep: result.unchangedMediaCount, remove: result.deletedMediaCount })}</p>{result.cleanupPending && <p className="warning-banner">{t("publish.cleanupPending")}</p>}</div></div>}
          <div className="dialog-actions"><Button variant="ghost" disabled={busy === "publish"} onClick={() => setStep(2)}>{t("common.back")}</Button>{canCancel && <Button variant="secondary" onClick={() => void backend.cancelPublish()}>{t("common.cancel")}</Button>}{result ? <><Button variant="secondary" onClick={() => void navigator.clipboard.writeText(result.publicUrl)}><Copy size={15} />{t("publish.copyUrl")}</Button><Button variant="primary" onClick={() => void backend.openPublicWebsite(result.publicUrl)}><ExternalLink size={15} />{t("publish.openSite")}</Button></> : <Button variant="primary" disabled={busy !== null || blockers.length > 0} onClick={() => void publish()}>{busy === "publish" ? t("publish.publishing") : state?.deployment ? t("publish.update") : t("publish.publish")}</Button>}</div>
        </section>}
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>;
}
