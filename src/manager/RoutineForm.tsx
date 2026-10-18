import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { errorMessage, type RepeatType, type Routine, type RoutineInput } from "@/lib/api";
import { cn } from "@/lib/utils";
import { emptyInput, normalize, REPEAT_NAMES, SLOT_NAMES, SLOTS, todayString, toggleBit, toInput, validateInput, WEEKDAY_NAMES } from "./routines";

interface Props {
  initial?: Routine;
  onSubmit: (input: RoutineInput) => Promise<void>;
  onCancel?: () => void;
}

const REPEATS: RepeatType[] = ["daily", "weekdays", "monthly", "once"];
const MONTH_DAYS = Array.from({ length: 31 }, (_, i) => i + 1);

export function RoutineForm({ initial, onSubmit, onCancel }: Props) {
  const [input, setInput] = useState<RoutineInput>(() => (initial ? toInput(initial) : emptyInput()));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const set = (patch: Partial<RoutineInput>) => {
    setInput((i) => ({ ...i, ...patch }));
    setError(null);
  };

  async function submit() {
    if (busy) return;
    const normalized = normalize(input);
    const message = validateInput(normalized);
    if (message) {
      setError(message);
      return;
    }
    setBusy(true);
    try {
      await onSubmit(normalized);
      if (!initial) setInput(emptyInput());
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-2.5 rounded-xl bg-soft/50 p-3.5">
      <Input
        aria-label="루틴 이름"
        placeholder="루틴 이름 (예: 급식 지도)"
        value={input.title}
        maxLength={40}
        onChange={(e) => set({ title: e.target.value })}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) {
            e.preventDefault();
            void submit();
          }
        }}
      />
      <div role="radiogroup" aria-label="반복" className="flex overflow-hidden rounded-lg border border-input bg-background text-xs">
        {REPEATS.map((t) => (
          <button
            key={t}
            type="button"
            role="radio"
            aria-checked={input.repeatType === t}
            onClick={() =>
              set({
                repeatType: t,
                onceDate: t === "once" ? (input.onceDate ?? todayString()) : input.onceDate,
                monthDay: t === "monthly" ? (input.monthDay ?? Number(todayString().slice(8))) : input.monthDay,
              })
            }
            className={cn("flex-1 py-1.5", input.repeatType === t ? "bg-soft font-medium text-ink" : "text-muted-foreground hover:bg-muted")}
          >
            {REPEAT_NAMES[t]}
          </button>
        ))}
      </div>
      {input.repeatType === "weekdays" && (
        <div className="flex gap-1">
          {WEEKDAY_NAMES.map((name, i) => {
            const on = (input.weekdays & (1 << i)) !== 0;
            return (
              <button
                key={name}
                type="button"
                aria-pressed={on}
                onClick={() => set({ weekdays: toggleBit(input.weekdays, i) })}
                className={cn(
                  "flex-1 rounded-md py-1 text-xs",
                  on ? "bg-soft-3 text-ink" : "border border-input bg-background text-muted-foreground",
                )}
              >
                {name}
              </button>
            );
          })}
        </div>
      )}
      {input.repeatType === "monthly" && (
        <div className="flex items-center gap-2 text-xs">
          <select
            aria-label="매월 날짜"
            value={input.monthDay ?? ""}
            onChange={(e) => set({ monthDay: Number(e.target.value) })}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            {MONTH_DAYS.map((d) => (
              <option key={d} value={d}>
                {d}일
              </option>
            ))}
            <option value={0}>마지막 평일</option>
          </select>
          <span className="text-muted-foreground">주말 · 공휴일이면 바로 앞 평일에 보여요</span>
        </div>
      )}
      {input.repeatType !== "once" && (
        <details className="text-xs" open={Boolean(input.activeFrom || input.activeUntil)}>
          <summary className="cursor-pointer text-muted-foreground select-none">기간 정하기 (예: 1학기, 방과후 운영 기간)</summary>
          <div className="mt-1.5 flex items-center gap-2">
            <Input type="date" aria-label="기간 시작일" value={input.activeFrom ?? ""} onChange={(e) => set({ activeFrom: e.target.value || null })} />
            <span className="text-muted-foreground">~</span>
            <Input type="date" aria-label="기간 끝나는 날" value={input.activeUntil ?? ""} onChange={(e) => set({ activeUntil: e.target.value || null })} />
          </div>
        </details>
      )}
      {input.repeatType === "once" && (
        <Input type="date" aria-label="날짜" value={input.onceDate ?? ""} onChange={(e) => set({ onceDate: e.target.value || null })} />
      )}
      <div role="radiogroup" aria-label="시간대" className="flex items-center gap-1 text-xs">
        <span className="mr-1 shrink-0 text-muted-foreground">시간대</span>
        {[null, ...SLOTS].map((s) => (
          <button
            key={s ?? "none"}
            type="button"
            role="radio"
            aria-checked={input.slot === s}
            onClick={() => set({ slot: s })}
            className={cn(
              "flex-1 rounded-md py-1",
              input.slot === s ? "bg-soft-3 text-ink" : "border border-input bg-background text-muted-foreground hover:bg-muted",
            )}
          >
            {s ? SLOT_NAMES[s] : "언제든"}
          </button>
        ))}
      </div>
      <div className="flex gap-2">
        <Input
          type="time"
          aria-label="마감 시각"
          className="w-32 shrink-0"
          value={input.dueTime ?? ""}
          onChange={(e) => set({ dueTime: e.target.value || null })}
        />
        <Input
          aria-label="바로가기"
          placeholder="바로가기: https://… 또는 C:\…\프로그램.exe"
          value={input.link ?? ""}
          onChange={(e) => set({ link: e.target.value })}
        />
      </div>
      {error && (
        <p role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        {onCancel && (
          <Button variant="ghost" size="sm" onClick={onCancel}>
            취소
          </Button>
        )}
        <Button size="sm" disabled={busy} onClick={() => void submit()}>
          {initial ? "저장" : "추가"}
        </Button>
      </div>
    </div>
  );
}
