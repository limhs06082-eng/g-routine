import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { compareVersions, unseenNotes } from "@/lib/changelog";
import { useAutoResize, useData, useSettings } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { Setup } from "./Setup";
import { TodayPanel } from "./TodayPanel";
import { WhatsNew } from "./WhatsNew";

export function WidgetApp() {
  const { data: status, error, reload } = useData(api.status);
  const ready = status?.ready ?? false;
  const { settings, update } = useSettings(ready);
  const { ref, capped } = useAutoResize<HTMLDivElement>();
  const pinned = settings?.alwaysOnTop ?? false;
  const mini = ready && (settings?.miniMode ?? false);
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);
  // 업데이트 뒤 아직 보지 않은 '바뀐 점'. 보여 줄 것이 없으면 조용히 지금 버전을 본 것으로 적는다.
  // 본 버전이 지금보다 새것이면(복원 프로그램이 예전 버전으로 되돌린 PC) 아무것도 하지 않는다.
  const seen = settings?.seenVersion ?? null;
  const newer = Boolean(settings && version && (seen === null || compareVersions(version, seen) > 0));
  const notes = newer && version ? unseenNotes(seen, version) : [];
  const markSeen = newer ? version : null;
  useEffect(() => {
    if (markSeen && notes.length === 0) void update("seen_version", markSeen).catch(() => {});
  }, [markSeen, notes.length, update]);
  const panel = (
    <TodayPanel
      pinned={pinned}
      onTogglePin={() => update("always_on_top", String(!pinned))}
      mini={mini}
      onToggleMini={() => update("mini_mode", String(!mini))}
      shortcut={status?.shortcut ?? false}
      banner={
        notes.length > 0 && markSeen ? <WhatsNew notes={notes} onClose={() => void update("seen_version", markSeen).catch(() => {})} /> : null
      }
    />
  );

  // 바깥 div: 창 높이. 내용이 화면(작업 영역) 상한을 넘었을 때만 스크롤을 켠다.
  // 안쪽 div(ref): 내용의 실제 높이. 이 높이를 Rust에 알려 창을 늘리고 줄인다.
  // 미니 모드에서는 카드 없이 알약만 오른쪽에 둔다 (창 너비는 그대로라 저장된 위치가 어긋나지 않는다).
  return (
    <div className={cn("widget-scroll h-screen", capped ? "overflow-y-auto" : "overflow-hidden")}>
      <div ref={ref} className={cn("p-1.5", mini && "flex justify-end")}>
        {mini ? (
          panel
        ) : (
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
              panel
            ) : (
              <Setup status={status} />
            )}
          </div>
        )}
      </div>
    </div>
  );
}
