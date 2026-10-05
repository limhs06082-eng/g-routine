import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useAutoResize, useData, useSettings } from "@/lib/hooks";
import { Setup } from "./Setup";
import { TodayPanel } from "./TodayPanel";

export function WidgetApp() {
  const { data: status, error, reload } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);
  const ref = useAutoResize<HTMLDivElement>();
  const pinned = settings?.alwaysOnTop ?? false;

  return (
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
  );
}
