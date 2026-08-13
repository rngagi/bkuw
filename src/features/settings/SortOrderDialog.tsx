import * as Dialog from "@radix-ui/react-dialog";
import { ArrowDown, ArrowUp, GripVertical, Plus, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { createId, type ManualSortItem, type ManualSortLayout, type ProjectSnapshot } from "../../types/domain";

interface Props {
  open: boolean;
  snapshot: ProjectSnapshot;
  onOpenChange(open: boolean): void;
  onSave(layout: ManualSortLayout): Promise<void>;
}

export function SortOrderDialog({ open, snapshot, onOpenChange, onSave }: Props) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ManualSortItem[]>([]);
  const [heading, setHeading] = useState("");
  const [dragged, setDragged] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!open) return;
    const present = new Set(snapshot.manualSortLayout.items.filter((item) => item.kind === "entry").map((item) => item.entryId));
    setItems([
      ...snapshot.manualSortLayout.items,
      ...snapshot.entries.filter((entry) => !present.has(entry.id)).map((entry) => ({ kind: "entry" as const, entryId: entry.id })),
    ]);
    setHeading(""); setError(false);
  }, [open, snapshot]);

  function move(from: number, to: number) {
    if (to < 0 || to >= items.length || from === to) return;
    setItems((current) => {
      const next = [...current];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
  }

  function label(item: ManualSortItem) {
    if (item.kind === "heading") return item.label;
    const entry = snapshot.entries.find((candidate) => candidate.id === item.entryId);
    return entry?.primaryForm || t("workspace.untitled");
  }

  async function save() {
    setBusy(true); setError(false);
    try {
      await onSave({ version: 1, items });
      onOpenChange(false);
    } catch { setError(true); }
    finally { setBusy(false); }
  }

  return <Dialog.Root open={open} onOpenChange={onOpenChange}><Dialog.Portal><Dialog.Overlay className="dialog-overlay" /><Dialog.Content className="dialog-content sort-dialog">
    <div className="dialog-heading"><div><Dialog.Title>{t("sorting.manualTitle")}</Dialog.Title><Dialog.Description>{t("sorting.manualHelp")}</Dialog.Description></div><Dialog.Close asChild><Button size="icon" variant="ghost" aria-label={t("common.close")}><X size={17} /></Button></Dialog.Close></div>
    {error && <p className="error-banner">{t("error.generic")}</p>}
    {snapshot.manualSortLayout.items.length === 0 && snapshot.entries.length > 0 && <p className="manual-layout-notice" role="status">{t("sorting.missingLayout")}</p>}
    <div className="inline-field"><input value={heading} placeholder={t("sorting.headingPlaceholder")} onChange={(event) => setHeading(event.target.value)} /><Button type="button" size="small" disabled={!heading.trim()} onClick={() => { setItems((current) => [...current, { kind: "heading", id: createId(), label: heading.trim() }]); setHeading(""); }}><Plus size={14} />{t("sorting.addHeading")}</Button></div>
    <div className="sort-layout" role="list">
      {items.map((item, index) => <div role="listitem" draggable key={item.kind === "heading" ? `h-${item.id}` : `e-${item.entryId}`} className={`sort-item ${item.kind}`} onDragStart={() => setDragged(index)} onDragOver={(event) => event.preventDefault()} onDrop={() => { if (dragged !== null) move(dragged, index); setDragged(null); }}>
        <GripVertical size={16} aria-hidden="true" /><span>{label(item)}</span>{item.kind === "entry" && snapshot.entries.find((entry) => entry.id === item.entryId)?.manualOrderPending && <small>{t("sorting.pending")}</small>}
        <div className="row-actions"><Button type="button" size="icon" variant="ghost" disabled={index === 0} onClick={() => move(index, index - 1)} aria-label={t("common.moveUp")}><ArrowUp size={14} /></Button><Button type="button" size="icon" variant="ghost" disabled={index === items.length - 1} onClick={() => move(index, index + 1)} aria-label={t("common.moveDown")}><ArrowDown size={14} /></Button>{item.kind === "heading" && <Button type="button" size="icon" variant="ghost" onClick={() => setItems((current) => current.filter((_, itemIndex) => itemIndex !== index))} aria-label={t("common.remove")}><Trash2 size={14} /></Button>}</div>
      </div>)}
    </div>
    <div className="dialog-actions"><Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button><Button variant="primary" disabled={busy} onClick={() => void save()}>{busy ? t("common.loading") : t("common.save")}</Button></div>
  </Dialog.Content></Dialog.Portal></Dialog.Root>;
}
