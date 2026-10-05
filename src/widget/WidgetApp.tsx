import { api } from "@/lib/api";
import { useAutoResize, useData, useSettings } from "@/lib/hooks";
import { Setup } from "./Setup";
import { TodayPanel } from "./TodayPanel";

export function WidgetApp() {
  const { data: status } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);
  const ref = useAutoResize<HTMLDivElement>();
  const pinned = settings?.alwaysOnTop ?? false;

  return (
    <div ref={ref} className="p-1.5">
      <div className="overflow-hidden rounded-xl border border-border bg-background shadow-[0_2px_10px_rgba(44,44,42,0.10)]">
        {!status ? (
          <div className="h-28" />
        ) : ready ? (
          <TodayPanel pinned={pinned} onTogglePin={() => void update("always_on_top", String(!pinned))} />
        ) : (
          <Setup status={status} />
        )}
      </div>
    </div>
  );
}
