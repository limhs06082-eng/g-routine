import type { TodayView } from "@/lib/api";

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
