import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { displayWritingSystemText } from "../../lib/writingSystems";
import type { EntrySummary, WritingSystem } from "../../types/domain";

interface Props { entries: EntrySummary[]; writingSystems: WritingSystem[]; selectedId: string | null; hasQuery: boolean; onSelect(id: string): void; }

export function EntryList({ entries, writingSystems, selectedId, hasQuery, onSelect }: Props) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({ count: entries.length, getScrollElement: () => parentRef.current, estimateSize: () => 68, overscan: 8 });
  if (!entries.length) return <div className="empty-list">{t(hasQuery ? "workspace.noMatch" : "workspace.noEntries")}</div>;
  return (
    <div className="entry-list" ref={parentRef}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const entry = entries[row.index];
          const primary = writingSystems.find((system) => system.displayRole === "primary");
          const secondary = writingSystems.find((system) => system.displayRole === "secondary");
          return <button key={entry.id} className={cn("entry-list-item", selectedId === entry.id && "selected")} style={{ transform: `translateY(${row.start}px)`, height: row.size }} onClick={() => onSelect(entry.id)}><strong>{entry.primaryForm ? displayWritingSystemText(entry.primaryForm, primary) : t("workspace.untitled")}</strong>{entry.secondaryForm && <span>{displayWritingSystemText(entry.secondaryForm, secondary)}</span>}{entry.partsOfSpeech.length > 0 && <small>{entry.partsOfSpeech.join(" · ")}</small>}</button>;
        })}
      </div>
    </div>
  );
}
