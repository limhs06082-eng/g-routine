import type { RoutineInput } from "@/lib/api";
import {
  emptyInput,
  isAllowedLink,
  moveItem,
  normalize,
  repeatLabel,
  todayString,
  toggleBit,
  validateInput,
  weekdayLabel,
} from "./routines";

const base: RoutineInput = { title: "출결 확인", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null, slot: null };

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
  expect(emptyInput()).toEqual({ title: "", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null, slot: null });
});
