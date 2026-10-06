import type { DayItem, Slot, TodayView } from "@/lib/api";

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];
const pad = (n: number) => String(n).padStart(2, "0");

export function formatDayLabel(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  const wd = new Date(y, m - 1, d).getDay();
  return `${m}월 ${d}일 ${WEEKDAYS[wd]}요일`;
}

export function progress(v: TodayView) {
  const done = v.done.length;
  const total = done + v.pending.length;
  return { done, total, percent: total === 0 ? 0 : Math.round((done / total) * 100) };
}

const SLOT_ORDER: (Slot | null)[] = ["morning", "class", "after", null];
export const SLOT_TITLES: Record<Slot | "none", string> = { morning: "조회 전", class: "수업 중", after: "방과 후", none: "언제든" };

/**
 * 할 일을 시간대(조회 전 → 수업 중 → 방과 후 → 언제든)로 묶는다. 각 묶음 안의 순서는 그대로 둔다.
 * 시간대를 정한 할 일이 하나도 없으면 묶지 않고 null을 돌려준다 (예전처럼 한 줄 목록).
 */
export function groupBySlot(items: DayItem[]): { slot: Slot | null; items: DayItem[] }[] | null {
  if (!items.some((i) => i.slot)) return null;
  return SLOT_ORDER.map((slot) => ({ slot, items: items.filter((i) => (i.slot ?? null) === slot) })).filter((g) => g.items.length > 0);
}

export function markDone(v: TodayView, itemId: number, at: string): TodayView {
  const item = v.pending.find((i) => i.id === itemId);
  if (!item) return v;
  return {
    ...v,
    pending: v.pending.filter((i) => i.id !== itemId),
    done: [...v.done, { ...item, completedAt: at }],
  };
}

export function markPending(v: TodayView, itemId: number): TodayView {
  const item = v.done.find((i) => i.id === itemId);
  if (!item) return v;
  const pending = [...v.pending, { ...item, completedAt: null }].sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id);
  return { ...v, pending, done: v.done.filter((i) => i.id !== itemId) };
}

export function localTimestamp(d: Date = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export function formatTime(ts: string | null): string {
  return ts ? ts.slice(11, 16) : "";
}
