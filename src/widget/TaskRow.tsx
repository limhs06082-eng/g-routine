import { Check, ExternalLink } from "lucide-react";
import type { DayItem } from "@/lib/api";
import { Chip } from "@/components/Chip";
import { cn } from "@/lib/utils";

interface Props {
  item: DayItem;
  leaving: boolean;
  onCheck: () => void;
  onOpenLink: () => void;
}

export function TaskRow({ item, leaving, onCheck, onOpenLink }: Props) {
  return (
    <li
      className={cn(
        "flex items-center gap-2.5 rounded-lg px-2.5 py-2 transition-all duration-200 hover:bg-soft/60",
        leaving && "-translate-x-2 opacity-0",
      )}
    >
      <button
        type="button"
        aria-label={`${item.title} 완료`}
        onClick={onCheck}
        className={cn(
          "flex size-[18px] shrink-0 items-center justify-center rounded-full border-[1.5px] border-soft-3 bg-background transition-colors hover:bg-soft",
          leaving && "bg-soft-3",
        )}
      >
        {leaving && <Check className="size-3 text-white" strokeWidth={3} />}
      </button>
      <span className="min-w-0 flex-1 truncate text-[13px]">{item.title}</span>
      {item.repeatType === "once" && <Chip kind="once">오늘만</Chip>}
      {item.dueTime &&
        (item.overdue ? (
          <Chip kind="overdue" label={`마감 ${item.dueTime} 지남`}>
            {item.dueTime}
          </Chip>
        ) : (
          <Chip kind="due">{item.dueTime}</Chip>
        ))}
      {item.hasLink && (
        <button
          type="button"
          aria-label={`${item.title} 바로가기 열기`}
          onClick={onOpenLink}
          className="rounded p-0.5 text-primary hover:bg-soft"
        >
          <ExternalLink className="size-3.5" />
        </button>
      )}
    </li>
  );
}
