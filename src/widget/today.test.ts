import type { DayItem, TodayView } from "@/lib/api";
import { formatDayLabel, formatTime, localTimestamp, markDone, markPending, progress } from "./today";

const item = (id: number, title: string, extra: Partial<DayItem> = {}): DayItem => ({
  id,
  day: "2026-10-05",
  routineId: id,
  title,
  sortOrder: id,
  completedAt: null,
  repeatType: "daily",
  dueTime: null,
  hasLink: false,
  overdue: false,
  ...extra,
});

const view: TodayView = { day: "2026-10-05", rest: null, pending: [item(1, "A"), item(2, "B")], done: [] };

test("formats Korean day label", () => {
  expect(formatDayLabel("2026-10-05")).toBe("10월 5일 월요일");
  expect(formatDayLabel("2026-10-11")).toBe("10월 11일 일요일");
});

test("progress counts done over total", () => {
  expect(progress(view)).toEqual({ done: 0, total: 2, percent: 0 });
  expect(progress(markDone(view, 1, "2026-10-05T08:00:00"))).toEqual({ done: 1, total: 2, percent: 50 });
  expect(progress({ ...view, pending: [] })).toEqual({ done: 0, total: 0, percent: 0 });
});

test("markDone and markPending move items keeping sort order", () => {
  const done = markDone(view, 1, "2026-10-05T08:00:00");
  expect(done.pending.map((i) => i.id)).toEqual([2]);
  expect(done.done[0]).toMatchObject({ id: 1, completedAt: "2026-10-05T08:00:00" });
  const back = markPending(done, 1);
  expect(back.pending.map((i) => i.id)).toEqual([1, 2]);
  expect(back.done).toEqual([]);
  expect(markDone(view, 99, "x")).toBe(view);
});

test("time helpers", () => {
  expect(localTimestamp(new Date(2026, 9, 5, 8, 7, 3))).toBe("2026-10-05T08:07:03");
  expect(formatTime("2026-10-05T08:47:12")).toBe("08:47");
  expect(formatTime(null)).toBe("");
});
