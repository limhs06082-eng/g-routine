import { Minimize2, Minus, Pin, PinOff, Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";

interface Props {
  dayLabel: string;
  pinned: boolean;
  onTogglePin: () => void;
  onShrink: () => void;
}

export function WidgetHeader({ dayLabel, pinned, onTogglePin, onShrink }: Props) {
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
        <Button variant="ghost" size="icon-sm" aria-label="작게 보기" title="작게 보기" onClick={onShrink}>
          <Minimize2 className="size-3.5 text-muted-foreground" />
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="관리 창 열기" onClick={() => void api.openManager()}>
          <SettingsIcon className="size-3.5 text-muted-foreground" />
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label="위젯 숨기기" title="숨기기 (Ctrl+Alt+G로 다시 보기)" onClick={() => void api.hideWidget()}>
          <Minus className="size-3.5 text-muted-foreground" />
        </Button>
      </div>
    </div>
  );
}
