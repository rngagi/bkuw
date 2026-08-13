import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { displayWritingSystemText } from "../../lib/writingSystems";
import type { EntrySummary, WritingSystem } from "../../types/domain";

interface Props { entries: EntrySummary[]; writingSystems: WritingSystem[]; selectedId: string | null; hasQuery: boolean; onSelect(id: string): void; }

function summarizedSenses(entry: EntrySummary) {
  return entry.senses.filter((sense) => sense.partOfSpeech || sense.gloss);
}

export function EntryList({ entries, writingSystems, selectedId, hasQuery, onSelect }: Props) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const primary = writingSystems.find((system) => system.displayRole === "primary");
  const secondary = writingSystems.find((system) => system.displayRole === "secondary");
  const rows = useMemo(() => entries.flatMap((entry, index) => {
    const previous = index > 0 ? entries[index - 1].sectionLabel : null;
    return entry.sectionLabel && entry.sectionLabel !== previous
      ? [{ kind: "heading" as const, label: entry.sectionLabel }, { kind: "entry" as const, entry }]
      : [{ kind: "entry" as const, entry }];
  }), [entries]);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const row = rows[index];
      if (!row || row.kind === "heading") return 38;
      const hasDistinctSecondary = Boolean(row.entry.secondaryForm)
        && secondary?.id !== row.entry.pronunciationWritingSystemId;
      const senseRows = Math.min(summarizedSenses(row.entry).length, 2);
      return 62 + Math.max(0, senseRows - 1) * 18 + (hasDistinctSecondary ? 18 : 0);
    },
    overscan: 8,
  });
  if (!entries.length) return <div className="empty-list">{t(hasQuery ? "workspace.noMatch" : "workspace.noEntries")}</div>;
  return (
    <div className="entry-list" ref={parentRef}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const item = rows[row.index];
          if (item.kind === "heading") return <div key={`heading-${row.index}-${item.label}`} className="entry-list-heading" style={{ transform: `translateY(${row.start}px)`, height: row.size }}>{item.label}</div>;
          const entry = item.entry;
          const pronunciationSystem = writingSystems.find(
            (system) => system.id === entry.pronunciationWritingSystemId,
          );
          const hasDistinctSecondary = Boolean(entry.secondaryForm)
            && secondary?.id !== entry.pronunciationWritingSystemId;
          const senses = summarizedSenses(entry);
          return <button key={entry.id} className={cn("entry-list-item", selectedId === entry.id && "selected")} style={{ transform: `translateY(${row.start}px)`, height: row.size }} onClick={() => onSelect(entry.id)}>
            <div className="entry-list-headword"><strong>{entry.primaryForm ? displayWritingSystemText(entry.primaryForm, primary) : t("workspace.untitled")}{entry.manualOrderPending && <span className="pending-order" title={t("sorting.pendingHelp")}> •</span>}</strong>{entry.pronunciationForm && <span>{displayWritingSystemText(entry.pronunciationForm, pronunciationSystem)}</span>}</div>
            {hasDistinctSecondary && <span>{displayWritingSystemText(entry.secondaryForm!, secondary)}</span>}
            {senses.slice(0, 2).map((sense, index) => <small className="entry-sense-summary" key={`${entry.id}-sense-${index}`}>{sense.partOfSpeech && <span className="entry-sense-pos">{sense.partOfSpeech}</span>}{sense.gloss && <span className="entry-sense-gloss">{sense.gloss}</span>}{senses.length > 2 && index === 1 && <span className="entry-sense-more">{t("workspace.moreSenses", { count: senses.length })}</span>}</small>)}
          </button>;
        })}
      </div>
    </div>
  );
}
