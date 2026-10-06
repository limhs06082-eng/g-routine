import { Check, GripVertical } from "lucide-react";
import type { TodayView } from "@/lib/api";
import { cn } from "@/lib/utils";
import { progress } from "./today";

/** 미니 모드: 알약 모양으로 진행 상황만 보여 준다. 누르면 원래 크기로 돌아간다. */
export function MiniPill({ view, onExpand }: { view: TodayView; onExpand: () => void }) {
  const { done, total } = progress(view);
  const late = view.pending.filter((i) => i.overdue).length;
  const allDone = total > 0 && done === total;
  const text = total === 0 ? (view.rest ? "쉬는 날" : "할 일 없음") : allDone ? "모두 완료" : `${done}/${total}`;
  const label = `크게 보기 · 오늘 할 일 ${total}개 중 ${done}개 완료${late > 0 ? ` · 마감 지난 일 ${late}개` : ""}`;

  return (
    <div data-mini-pill className="flex items-center rounded-full border border-border bg-background py-0.5 pr-0.5 pl-1 shadow-[0_2px_10px_rgba(44,44,42,0.10)]">
      <span data-tauri-drag-region title="끌어서 옮기기" className="cursor-grab self-stretch px-1.5 py-1 text-muted-foreground">
        <GripVertical className="pointer-events-none size-3.5" />
      </span>
      <button
        type="button"
        aria-label={label}
        title="크게 보기"
        onClick={onExpand}
        className="flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium hover:bg-muted"
      >
        <Check className={cn("size-3.5", allDone ? "text-success" : "text-primary")} strokeWidth={3} />
        <span>{text}</span>
        {late > 0 && <span className="rounded-full bg-danger-soft px-1.5 text-[10px] leading-4 text-danger">{late}</span>}
      </button>
    </div>
  );
}
