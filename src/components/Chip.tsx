import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export type ChipKind = "daily" | "weekdays" | "once" | "due" | "overdue";

const STYLES: Record<ChipKind, string> = {
  daily: "bg-chip-daily text-chip-daily-ink",
  weekdays: "bg-chip-weekdays text-chip-weekdays-ink",
  once: "bg-chip-once text-chip-once-ink",
  due: "bg-chip-due text-chip-due-ink",
  overdue: "bg-danger-soft text-danger font-medium",
};

export function Chip({ kind, children, label, className }: { kind: ChipKind; children: ReactNode; label?: string; className?: string }) {
  return (
    <span
      aria-label={label}
      title={label}
      className={cn("inline-flex shrink-0 items-center rounded-full px-2 py-px text-[11px] leading-4", STYLES[kind], className)}
    >
      {children}
    </span>
  );
}
