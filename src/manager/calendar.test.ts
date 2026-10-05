import { buildMonthGrid, formatDayTitle, formatHour, levelOf, shiftMonth } from "./calendar";

test("month grid starts on Sunday with leading blanks", () => {
  const cells = buildMonthGrid(2026, 10); // 2026-10-01은 목요일
  expect(cells.slice(0, 5)).toEqual([null, null, null, null, "2026-10-01"]);
  expect(cells).toHaveLength(35);
  expect(cells[cells.length - 1]).toBe("2026-10-31");
});

test("completion level buckets", () => {
  expect(levelOf(undefined)).toBe(-1);
  expect(levelOf({ day: "d", total: 0, completed: 0 })).toBe(-1);
  expect(levelOf({ day: "d", total: 6, completed: 0 })).toBe(0);
  expect(levelOf({ day: "d", total: 6, completed: 2 })).toBe(1);
  expect(levelOf({ day: "d", total: 6, completed: 3 })).toBe(2);
  expect(levelOf({ day: "d", total: 6, completed: 6 })).toBe(3);
});

test("month shifting wraps years", () => {
  expect(shiftMonth(2026, 12, 1)).toEqual({ year: 2027, month: 1 });
  expect(shiftMonth(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
  expect(shiftMonth(2026, 10, 1)).toEqual({ year: 2026, month: 11 });
});

test("labels", () => {
  expect(formatDayTitle("2026-10-02")).toBe("10월 2일 (금)");
  expect(formatHour(0)).toBe("자정 (0시)");
  expect(formatHour(4)).toBe("오전 4시");
  expect(formatHour(12)).toBe("낮 12시");
});
