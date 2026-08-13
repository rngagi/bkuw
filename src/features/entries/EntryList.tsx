import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { displayWritingSystemText } from "../../lib/writingSystems";
import type { EntrySummary, WritingSystem } from "../../types/domain";

interface Props { entries: EntrySummary[]; writingSystems: WritingSystem[]; selectedId: string | null; hasQuery: boolean; onSelect(id: string): void; }

export function EntryList({ entries, writingSystems, selectedId, hasQuery, onSelect }: Props) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => entries.flatMap((entry, index) => {
    const previous = index > 0 ? entries[index - 1].sectionLabel : null;
    return entry.sectionLabel && entry.sectionLabel !== previous
      ? [{ kind: "heading" as const, label: entry.sectionLabel }, { kind: "entry" as const, entry }]
      : [{ kind: "entry" as const, entry }];
  }), [entries]);
  const virtualizer = useVirtualizer({ count: rows.length, getScrollElement: () => parentRef.current, estimateSize: (index) => rows[index]?.kind === "heading" ? 38 : 68, overscan: 8 });
  if (!entries.length) return <div className="empty-list">{t(hasQuery ? "workspace.noMatch" : "workspace.noEntries")}</div>;
  return (
    <div className="entry-list" ref={parentRef}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const item = rows[row.index];
          if (item.kind === "heading") return <div key={`heading-${row.index}-${item.label}`} className="entry-list-heading" style={{ transform: `translateY(${row.start}px)`, height: row.size }}>{item.label}</div>;
          const entry = item.entry;
          const primary = writingSystems.find((system) => system.displayRole === "primary");
          const secondary = writingSystems.find((system) => system.displayRole === "secondary");
          return <button key={entry.id} className={cn("entry-list-item", selectedId === entry.id && "selected")} style={{ transform: `translateY(${row.start}px)`, height: row.size }} onClick={() => onSelect(entry.id)}><strong>{entry.primaryForm ? displayWritingSystemText(entry.primaryForm, primary) : t("workspace.untitled")}{entry.manualOrderPending && <span className="pending-order" title={t("sorting.pendingHelp")}> •</span>}</strong>{entry.secondaryForm && <span>{displayWritingSystemText(entry.secondaryForm, secondary)}</span>}{entry.partsOfSpeech.length > 0 && <small>{entry.partsOfSpeech.join(" · ")}</small>}</button>;
        })}
      </div>
    </div>
  );
}
