import { Check } from "lucide-react";
import type { DayItem } from "@/lib/api";
import { formatTime } from "./today";

export function DoneRow({ item, onUncheck }: { item: DayItem; onUncheck: () => void }) {
  return (
    <li className="flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-muted">
      <button
        type="button"
        aria-label={`${item.title} 완료 취소`}
        onClick={onUncheck}
        className="flex size-[18px] shrink-0 items-center justify-center rounded-full bg-soft-3"
      >
        <Check className="size-3 text-white" strokeWidth={3} />
      </button>
      <span className="min-w-0 flex-1 truncate text-[13px] text-muted-foreground line-through">{item.title}</span>
      <span className="text-[11px] text-muted-foreground">{formatTime(item.completedAt)}</span>
    </li>
  );
}
