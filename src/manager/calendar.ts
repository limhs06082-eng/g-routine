import type { DaySummary } from "@/lib/api";

const pad = (n: number) => String(n).padStart(2, "0");
const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];

export type Level = -1 | 0 | 1 | 2 | 3;

export function buildMonthGrid(year: number, month: number): (string | null)[] {
  const first = new Date(year, month - 1, 1).getDay();
  const days = new Date(year, month, 0).getDate();
  const cells: (string | null)[] = Array.from({ length: first }, () => null);
  for (let d = 1; d <= days; d++) cells.push(`${year}-${pad(month)}-${pad(d)}`);
  return cells;
}

export function levelOf(s?: DaySummary): Level {
  if (!s || s.total === 0) return -1;
  if (s.completed === 0) return 0;
  if (s.completed >= s.total) return 3;
  return s.completed * 2 >= s.total ? 2 : 1;
}

export function shiftMonth(year: number, month: number, delta: number) {
  const index = year * 12 + (month - 1) + delta;
  return { year: Math.floor(index / 12), month: (index % 12) + 1 };
}

export function formatDayTitle(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return `${m}월 ${d}일 (${WEEKDAYS[new Date(y, m - 1, d).getDay()]})`;
}

export function formatHour(h: number): string {
  if (h === 0) return "자정 (0시)";
  if (h === 12) return "낮 12시";
  return h < 12 ? `오전 ${h}시` : `오후 ${h - 12}시`;
}
