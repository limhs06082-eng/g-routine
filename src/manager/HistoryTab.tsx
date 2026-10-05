import { useState } from "react";
import { Check, ChevronLeft, ChevronRight, Circle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useData } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { formatTime } from "@/widget/today";
import { buildMonthGrid, formatDayTitle, levelOf, shiftMonth, type Level } from "./calendar";
import { todayString } from "./routines";

const LEVEL_CLASS: Record<Level, string> = {
  [-1]: "text-muted-foreground/60",
  0: "bg-muted text-muted-foreground",
  1: "bg-soft text-ink",
  2: "bg-soft-2 text-ink",
  3: "bg-soft-3 text-ink font-medium",
};

function LoadError({ text, onRetry }: { text: string; onRetry: () => void }) {
  return (
    <div className="flex items-center gap-2 text-xs text-muted-foreground">
      <span>{text}</span>
      <Button size="sm" variant="outline" onClick={onRetry}>
        다시 시도
      </Button>
    </div>
  );
}

export function HistoryTab() {
  const now = new Date();
  const [ym, setYm] = useState({ year: now.getFullYear(), month: now.getMonth() + 1 });
  const [selected, setSelected] = useState(todayString());
  // 달/날짜가 바뀐 직후 이전 응답이 남아 있어도 다른 날 기록으로 보이지 않도록 키를 함께 보관한다.
  const month = useData(
    async () => ({ ...ym, summary: await api.historyMonth(ym.year, ym.month) }),
    [ym.year, ym.month],
  );
  const dayData = useData(async () => ({ day: selected, items: await api.historyDay(selected) }), [selected]);

  const monthFresh = month.data && month.data.year === ym.year && month.data.month === ym.month ? month.data : null;
  const dayFresh = dayData.data && dayData.data.day === selected ? dayData.data : null;

  const byDay = new Map((monthFresh?.summary ?? []).map((s) => [s.day, s]));
  const cells = buildMonthGrid(ym.year, ym.month);
  const list = dayFresh?.items ?? [];
  const doneCount = list.filter((i) => i.completedAt).length;

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_280px] gap-6">
      <section>
        <div className="mb-3 flex items-center justify-between">
          <Button variant="ghost" size="icon" aria-label="이전 달" onClick={() => setYm(shiftMonth(ym.year, ym.month, -1))}>
            <ChevronLeft />
          </Button>
          <span className="text-sm font-semibold">
            {ym.year}년 {ym.month}월
          </span>
          <Button variant="ghost" size="icon" aria-label="다음 달" onClick={() => setYm(shiftMonth(ym.year, ym.month, 1))}>
            <ChevronRight />
          </Button>
        </div>
        {!monthFresh && month.error && (
          <div className="mb-2">
            <LoadError text="기록을 불러오지 못했어요." onRetry={() => void month.reload()} />
          </div>
        )}
        <div className="grid grid-cols-7 gap-1 text-center text-xs">
          {["일", "월", "화", "수", "목", "금", "토"].map((d) => (
            <div key={d} className="py-1 text-muted-foreground">
              {d}
            </div>
          ))}
          {cells.map((day, i) =>
            day ? (
              <button
                key={day}
                type="button"
                aria-label={formatDayTitle(day)}
                aria-pressed={selected === day}
                onClick={() => setSelected(day)}
                className={cn(
                  "aspect-square rounded-lg py-1 transition-colors hover:ring-1 hover:ring-soft-3",
                  LEVEL_CLASS[levelOf(byDay.get(day))],
                  selected === day && "ring-2 ring-strong",
                )}
              >
                {Number(day.slice(8))}
              </button>
            ) : (
              <div key={`blank-${i}`} />
            ),
          )}
        </div>
        <div className="mt-3 flex items-center justify-end gap-1.5 text-[11px] text-muted-foreground">
          적음
          <span className="size-3 rounded bg-muted" />
          <span className="size-3 rounded bg-soft" />
          <span className="size-3 rounded bg-soft-2" />
          <span className="size-3 rounded bg-soft-3" />
          모두 완료
        </div>
      </section>
      <section className="rounded-xl border border-border p-4">
        <h2 className="text-sm font-semibold">{formatDayTitle(selected)}</h2>
        {!dayFresh ? (
          dayData.error && (
            <div className="mt-3">
              <LoadError text="기록을 불러오지 못했어요." onRetry={() => void dayData.reload()} />
            </div>
          )
        ) : list.length === 0 ? (
          <p className="mt-3 text-xs text-muted-foreground">이날은 기록이 없어요.</p>
        ) : (
          <>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {list.length}개 중 {doneCount}개 완료
            </p>
            <ul className="mt-3 flex flex-col gap-1.5 text-[13px]">
              {list.map((item) => (
                <li key={item.id} className="flex items-center gap-2">
                  {item.completedAt ? (
                    <Check className="size-4 shrink-0 text-success" />
                  ) : (
                    <Circle className="size-4 shrink-0 text-input" />
                  )}
                  <span className={cn("min-w-0 flex-1 truncate", !item.completedAt && "text-muted-foreground")}>{item.title}</span>
                  <span className="text-[11px] text-muted-foreground">{item.completedAt ? formatTime(item.completedAt) : "미완료"}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
    </div>
  );
}
