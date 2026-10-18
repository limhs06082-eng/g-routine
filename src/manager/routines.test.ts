import type { RoutineInput } from "@/lib/api";
import {
  emptyInput,
  isAllowedLink,
  moveItem,
  normalize,
  periodLabel,
  repeatLabel,
  todayString,
  toggleBit,
  validateInput,
  weekdayLabel,
} from "./routines";

const base: RoutineInput = { title: "출결 확인", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null, slot: null, monthDay: null, activeFrom: null, activeUntil: null };

test("weekday and repeat labels", () => {
  expect(weekdayLabel(5)).toBe("월·수");
  expect(weekdayLabel(16)).toBe("금");
  expect(repeatLabel({ repeatType: "daily", weekdays: 0, onceDate: null })).toBe("매일");
  expect(repeatLabel({ repeatType: "weekdays", weekdays: 17, onceDate: null })).toBe("월·금");
  expect(repeatLabel({ repeatType: "once", weekdays: 0, onceDate: "2026-10-05" })).toBe("10/5 하루");
});

test("bit toggle and list move", () => {
  expect(toggleBit(0, 0)).toBe(1);
  expect(toggleBit(5, 2)).toBe(1);
  expect(moveItem(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
  expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
});

test("link rules mirror the backend", () => {
  expect(isAllowedLink("https://www.neis.go.kr")).toBe(true);
  expect(isAllowedLink("C:\\Program Files\\app.exe")).toBe(true);
  expect(isAllowedLink("\\\\server\\share")).toBe(true);
  expect(isAllowedLink("https://")).toBe(false);
  expect(isAllowedLink("javascript:alert(1)")).toBe(false);
  expect(isAllowedLink("notepad.exe")).toBe(false);
});

test("validation messages", () => {
  expect(validateInput(base)).toBeNull();
  expect(validateInput({ ...base, title: "  " })).toBe("루틴 이름을 입력해 주세요");
  expect(validateInput({ ...base, title: "가".repeat(41) })).toBe("루틴 이름은 40자 이내로 입력해 주세요");
  expect(validateInput({ ...base, repeatType: "weekdays", weekdays: 0 })).toBe("요일을 하나 이상 골라 주세요");
  expect(validateInput({ ...base, repeatType: "once", onceDate: null })).toBe("날짜를 골라 주세요");
  expect(validateInput({ ...base, link: "ftp://x" })).toBe("바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요");
});

test("normalize trims and clears fields that do not apply", () => {
  expect(normalize({ ...base, title: " 수업 준비 ", weekdays: 3, link: "  ", dueTime: "" })).toEqual({
    ...base,
    title: "수업 준비",
    weekdays: 0,
    link: null,
    dueTime: null,
  });
  expect(normalize({ ...base, repeatType: "once", onceDate: null }).onceDate).toBe(todayString());
  expect(emptyInput()).toEqual({ title: "", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null, slot: null, monthDay: null, activeFrom: null, activeUntil: null });
});

test("monthly and period labels", () => {
  expect(repeatLabel({ repeatType: "monthly", weekdays: 0, onceDate: null, monthDay: 10 })).toBe("매월 10일");
  expect(repeatLabel({ repeatType: "monthly", weekdays: 0, onceDate: null, monthDay: 0 })).toBe("매월 마지막 평일");
  expect(periodLabel({ activeFrom: "2027-03-02", activeUntil: "2027-07-20" })).toBe("3/2~7/20");
  expect(periodLabel({ activeFrom: null, activeUntil: "2026-12-31" })).toBe("~12/31");
  expect(periodLabel({ activeFrom: null, activeUntil: null })).toBeNull();
});

test("normalize drops the month day and period where they do not apply", () => {
  const daily = normalize({ ...base, monthDay: 5, activeFrom: "2027-03-02", activeUntil: "" });
  expect([daily.monthDay, daily.activeFrom, daily.activeUntil]).toEqual([null, "2027-03-02", null]);
  const once = normalize({ ...base, repeatType: "once", onceDate: "2026-10-20", activeFrom: "2027-03-02" });
  expect(once.activeFrom).toBeNull();
  expect(validateInput({ ...base, repeatType: "monthly", monthDay: null })).toBe("매월 며칠인지 골라 주세요");
});
