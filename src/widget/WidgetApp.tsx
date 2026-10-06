import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useAutoResize, useData, useSettings } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { Setup } from "./Setup";
import { TodayPanel } from "./TodayPanel";

export function WidgetApp() {
  const { data: status, error, reload } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);
  const { ref, capped } = useAutoResize<HTMLDivElement>();
  const pinned = settings?.alwaysOnTop ?? false;

  // 바깥 div: 창 높이. 내용이 화면(작업 영역) 상한을 넘었을 때만 스크롤을 켠다.
  // 안쪽 div(ref): 내용의 실제 높이. 이 높이를 Rust에 알려 창을 늘리고 줄인다.
  return (
    <div className={cn("widget-scroll h-screen", capped ? "overflow-y-auto" : "overflow-hidden")}>
      <div ref={ref} className="p-1.5">
        <div className="overflow-hidden rounded-xl border border-border bg-background shadow-[0_2px_10px_rgba(44,44,42,0.10)]">
          {!status ? (
            error ? (
              <div className="flex flex-col items-center gap-2 px-4 py-6 text-center text-xs text-muted-foreground">
                <span>G-routine 상태를 불러오지 못했어요.</span>
                <Button size="sm" variant="outline" onClick={() => void reload()}>
                  다시 시도
                </Button>
              </div>
            ) : (
              <div className="h-28" />
            )
          ) : ready ? (
            <TodayPanel pinned={pinned} onTogglePin={() => update("always_on_top", String(!pinned))} />
          ) : (
            <Setup status={status} />
          )}
        </div>
      </div>
    </div>
  );
}
