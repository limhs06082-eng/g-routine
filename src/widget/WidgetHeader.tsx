import { Minus, Pin, PinOff, Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";

export function WidgetHeader({ dayLabel, pinned, onTogglePin }: { dayLabel: string; pinned: boolean; onTogglePin: () => void }) {
  return (
    <div data-tauri-drag-region className="flex cursor-default items-center justify-between px-3.5 pt-3 pb-2.5">
      <div data-tauri-drag-region>
        <div data-tauri-drag-region className="text-sm font-semibold">
          {dayLabel}
        </div>
        <div data-tauri-drag-region className="text-[11px] text-muted-foreground">
          G-routine
        </div>
      </div>
      <div className="flex items-center gap-0.5">
        <Button variant="ghost" size="icon-sm" aria-label={pinned ? "맨 위 고정 해제" : "맨 위에 고정"} onClick={onTogglePin}>
          {pinned ? <Pin className="size-3.5 text-primary" /> : <PinOff className="size-3.5 text-muted-foreground" />}
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="관리 창 열기" onClick={() => void api.openManager()}>
          <SettingsIcon className="size-3.5 text-muted-foreground" />
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="위젯 숨기기" onClick={() => void api.hideWidget()}>
          <Minus className="size-3.5 text-muted-foreground" />
        </Button>
      </div>
    </div>
  );
}
