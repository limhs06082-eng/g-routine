import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api, type Settings } from "@/lib/api";

function monthDay(day: string, withYear: boolean): string {
  const [y, m, d] = day.split("-").map(Number);
  return withYear ? `${y}년 ${m}월 ${d}일` : `${m}월 ${d}일`;
}

export function vacationLabel(start: string, end: string): string {
  // 겨울방학처럼 해를 넘기면 연도까지 적는다
  const withYear = start.slice(0, 4) !== end.slice(0, 4);
  return `${monthDay(start, withYear)} ~ ${monthDay(end, withYear)} 동안 반복 루틴을 쉬어요`;
}

interface Props {
  settings: Settings;
  /** SettingsTab의 실행 도우미: 실패하면 오류 문구를, 성공하면 ok 문구를 보여 준다 */
  run: (action: () => Promise<unknown>, ok?: string) => Promise<void>;
}

/** 방학 · 쉬는 기간: 정해 두면 그동안 반복 루틴을 띄우지 않는다 (지난 기록은 그대로) */
export function VacationSetting({ settings, run }: Props) {
  const [start, setStart] = useState(settings.vacationStart ?? "");
  const [end, setEnd] = useState(settings.vacationEnd ?? "");
  useEffect(() => {
    setStart(settings.vacationStart ?? "");
    setEnd(settings.vacationEnd ?? "");
  }, [settings.vacationStart, settings.vacationEnd]);
  const active = settings.vacationStart && settings.vacationEnd;

  const save = () =>
    run(async () => {
      if (!start || !end) throw new Error("시작일과 끝나는 날을 모두 골라 주세요");
      if (start > end) throw new Error("시작일이 끝나는 날보다 늦어요. 날짜를 다시 골라 주세요");
      await api.setVacation(start, end);
    }, "방학 기간을 저장했어요");
  const clear = () => run(() => api.setVacation(null, null), "방학 기간을 해제했어요");

  return (
    <div className="border-b border-border py-3">
      <div className="text-sm">방학 · 쉬는 기간</div>
      <div className="text-xs text-muted-foreground">
        {active
          ? vacationLabel(settings.vacationStart as string, settings.vacationEnd as string)
          : "기간을 정하면 그동안 반복 루틴을 띄우지 않아요. 지난 기록은 그대로 남아요"}
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <Input type="date" aria-label="방학 시작일" className="w-40" value={start} onChange={(e) => setStart(e.target.value)} />
        <span className="text-xs text-muted-foreground">~</span>
        <Input type="date" aria-label="방학 끝나는 날" className="w-40" value={end} onChange={(e) => setEnd(e.target.value)} />
        <Button size="sm" variant="outline" aria-label="방학 저장" onClick={() => void save()}>
          저장
        </Button>
        {active && (
          <Button size="sm" variant="ghost" aria-label="방학 해제" onClick={() => void clear()}>
            해제
          </Button>
        )}
      </div>
    </div>
  );
}
