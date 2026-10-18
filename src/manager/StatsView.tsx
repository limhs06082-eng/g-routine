import { useEffect, useState } from "react";
import { Download, Flame } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { api, errorMessage, type DaySummary } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { todayString } from "./routines";

const RANGES = [
  { days: 7, label: "최근 7일" },
  { days: 30, label: "최근 30일" },
  { days: 90, label: "최근 90일" },
] as const;

function shift(day: string, delta: number): string {
  const [y, m, d] = day.split("-").map(Number);
  return todayString(new Date(y, m - 1, d + delta));
}

export function percent(completed: number, total: number): number {
  return total === 0 ? 0 : Math.round((completed / total) * 100);
}

export interface Bar {
  key: string;
  label: string;
  total: number;
  completed: number;
}

/** 기간의 막대: 31일까지는 하루에 하나, 그보다 길면 월요일부터 한 주에 하나 */
export function buildBars(from: string, to: string, days: DaySummary[]): Bar[] {
  const byDay = new Map(days.map((d) => [d.day, d]));
  const dayList: string[] = [];
  for (let d = from; d <= to; d = shift(d, 1)) dayList.push(d);
  const label = (d: string) => `${Number(d.slice(5, 7))}/${Number(d.slice(8, 10))}`;
  if (dayList.length <= 31) {
    return dayList.map((d) => ({ key: d, label: label(d), total: byDay.get(d)?.total ?? 0, completed: byDay.get(d)?.completed ?? 0 }));
  }
  const weeks = new Map<string, Bar>();
  for (const d of dayList) {
    const [y, m, dd] = d.split("-").map(Number);
    const monday = shift(d, -((new Date(y, m - 1, dd).getDay() + 6) % 7));
    // 첫 주는 기간 시작일부터라 그날을 이름으로 쓴다
    const bar = weeks.get(monday) ?? { key: monday, label: `${label(monday < from ? from : monday)}주`, total: 0, completed: 0 };
    bar.total += byDay.get(d)?.total ?? 0;
    bar.completed += byDay.get(d)?.completed ?? 0;
    weeks.set(monday, bar);
  }
  return [...weeks.values()];
}

/** 완료 기록 탭의 '통계': 기간 달성률, 연속 달성, 날짜별 막대, 자주 놓치는 루틴, CSV 내보내기 */
export function StatsView() {
  const [days, setDays] = useState<number>(30);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  // 하루 시작 시각 전이면 오늘은 아직 전날이다 (달력 날짜로 하면 내일이 빈 막대로 들어간다)
  const [today, setToday] = useState(todayString());
  useEffect(() => {
    api
      .today()
      .then((v) => setToday(v.day))
      .catch(() => {});
  }, []);
  const to = today;
  const from = shift(to, -(days - 1));
  const { data, error, reload } = useData(() => api.historyStats(from, to), [from, to]);
  const stats = data && data.from === from && data.to === to ? data : null;

  async function exportCsv() {
    try {
      const path = await save({ defaultPath: `g-routine-기록-${from}_${to}.csv`, filters: [{ name: "CSV (엑셀)", extensions: ["csv"] }] });
      if (!path) return;
      await api.exportCsv(path, from, to);
      setMessage({ ok: true, text: "CSV 파일로 저장했어요. 엑셀에서 열 수 있어요" });
    } catch (e) {
      setMessage({ ok: false, text: errorMessage(e) });
    }
  }

  const bars = stats ? buildBars(from, to, stats.days) : [];
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <div role="radiogroup" aria-label="기간" className="flex overflow-hidden rounded-lg border border-input text-xs">
          {RANGES.map((r) => (
            <button
              key={r.days}
              type="button"
              role="radio"
              aria-checked={days === r.days}
              onClick={() => setDays(r.days)}
              className={cn("px-3 py-1.5", days === r.days ? "bg-soft font-medium text-ink" : "text-muted-foreground hover:bg-muted")}
            >
              {r.label}
            </button>
          ))}
        </div>
        <Button variant="outline" size="sm" className="ml-auto" onClick={() => void exportCsv()}>
          <Download />
          CSV로 저장 (엑셀)
        </Button>
      </div>
      {message && (
        <p role={message.ok ? "status" : "alert"} className={cn("text-xs", message.ok ? "text-strong" : "text-danger")}>
          {message.text}
        </p>
      )}
      {!stats ? (
        error && (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span>통계를 불러오지 못했어요.</span>
            <Button size="sm" variant="outline" onClick={() => void reload()}>
              다시 시도
            </Button>
          </div>
        )
      ) : stats.total === 0 ? (
        <p className="text-sm text-muted-foreground">이 기간에는 기록이 없어요.</p>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-xl border border-border p-4">
              <div className="text-xs text-muted-foreground">달성률</div>
              <div className="mt-1 text-2xl font-semibold text-strong">{percent(stats.completed, stats.total)}%</div>
              <div className="text-xs text-muted-foreground">
                {stats.total}개 중 {stats.completed}개 완료
              </div>
            </div>
            <div className="rounded-xl border border-border p-4">
              <div className="text-xs text-muted-foreground">연속 달성</div>
              <div className="mt-1 flex items-center gap-1 text-2xl font-semibold text-strong">
                {stats.streak > 0 && <Flame className="size-5 text-primary" />}
                {stats.streak}일
              </div>
              <div className="text-xs text-muted-foreground">모두 마친 날이 이어진 수 (쉬는 날은 건너뛰어요)</div>
            </div>
          </div>
          <section aria-label="날짜별 달성" className="rounded-xl border border-border p-4">
            <div className="mb-2 text-xs text-muted-foreground">{days > 31 ? "주별 달성" : "날짜별 달성"}</div>
            <div className="flex h-28 items-end gap-0.5">
              {bars.map((b) => (
                <div
                  key={b.key}
                  className="flex h-full min-w-0 flex-1 flex-col justify-end"
                  title={b.total ? `${b.label}: ${b.total}개 중 ${b.completed}개 (${percent(b.completed, b.total)}%)` : `${b.label}: 기록 없음`}
                >
                  <div
                    className={cn("w-full rounded-t", b.total === 0 ? "h-0.5 bg-muted" : b.completed >= b.total ? "bg-success" : "bg-soft-3")}
                    style={b.total ? { height: `${Math.max(4, percent(b.completed, b.total))}%` } : undefined}
                  />
                </div>
              ))}
            </div>
            <div className="mt-1 flex justify-between text-[11px] text-muted-foreground">
              <span>{bars[0]?.label}</span>
              <span>{bars[bars.length - 1]?.label}</span>
            </div>
          </section>
          <section aria-label="루틴별 달성률" className="rounded-xl border border-border p-4">
            <div className="mb-2 text-xs text-muted-foreground">루틴별 달성률 (자주 놓치는 일부터)</div>
            <ul className="flex flex-col gap-2 text-[13px]">
              {stats.routines.map((r) => (
                <li key={r.routineId} className="grid grid-cols-[minmax(0,1fr)_120px_56px] items-center gap-3">
                  <span className="truncate">{r.title}</span>
                  <div className="h-1.5 rounded-full bg-muted">
                    <div className="h-1.5 rounded-full bg-soft-3" style={{ width: `${percent(r.completed, r.total)}%` }} />
                  </div>
                  <span className="text-right text-xs text-muted-foreground">
                    {r.completed}/{r.total}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        </>
      )}
    </div>
  );
}
