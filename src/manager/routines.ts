import type { RepeatType, Routine, RoutineInput } from "@/lib/api";

export const WEEKDAY_NAMES = ["월", "화", "수", "목", "금", "토", "일"];
export const REPEAT_NAMES: Record<RepeatType, string> = { daily: "매일", weekdays: "요일 지정", once: "하루만" };

const pad = (n: number) => String(n).padStart(2, "0");

export function weekdayLabel(bits: number): string {
  return WEEKDAY_NAMES.filter((_, i) => (bits & (1 << i)) !== 0).join("·");
}

export function repeatLabel(r: Pick<Routine, "repeatType" | "weekdays" | "onceDate">): string {
  if (r.repeatType === "daily") return "매일";
  if (r.repeatType === "weekdays") return weekdayLabel(r.weekdays);
  if (!r.onceDate) return "하루";
  return `${Number(r.onceDate.slice(5, 7))}/${Number(r.onceDate.slice(8, 10))} 하루`;
}

export function toggleBit(bits: number, i: number): number {
  return bits ^ (1 << i);
}

export function moveItem<T>(list: T[], from: number, to: number): T[] {
  const next = [...list];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export function isAllowedLink(link: string): boolean {
  const lower = link.toLowerCase();
  for (const scheme of ["https://", "http://"]) {
    if (lower.startsWith(scheme)) return link.length > scheme.length;
  }
  return /^[a-zA-Z]:[\\/]/.test(link) || link.startsWith("\\\\");
}

export function todayString(d: Date = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function emptyInput(): RoutineInput {
  return { title: "", repeatType: "daily", weekdays: 0, onceDate: null, dueTime: null, link: null };
}

export function toInput(r: Routine): RoutineInput {
  return { title: r.title, repeatType: r.repeatType, weekdays: r.weekdays, onceDate: r.onceDate, dueTime: r.dueTime, link: r.link };
}

/** 저장 전 정리: 공백 제거, 반복 종류에 맞지 않는 값 비우기 */
export function normalize(input: RoutineInput): RoutineInput {
  return {
    title: input.title.trim(),
    repeatType: input.repeatType,
    weekdays: input.repeatType === "weekdays" ? input.weekdays : 0,
    onceDate: input.repeatType === "once" ? (input.onceDate ?? todayString()) : null,
    dueTime: input.dueTime ? input.dueTime : null,
    link: input.link?.trim() ? input.link.trim() : null,
  };
}

export function validateInput(input: RoutineInput): string | null {
  const title = input.title.trim();
  if (!title) return "루틴 이름을 입력해 주세요";
  if ([...title].length > 40) return "루틴 이름은 40자 이내로 입력해 주세요";
  if (input.repeatType === "weekdays" && (input.weekdays & 0x7f) === 0) return "요일을 하나 이상 골라 주세요";
  if (input.repeatType === "once" && !input.onceDate) return "날짜를 골라 주세요";
  if (input.link && input.link.trim() && !isAllowedLink(input.link.trim())) {
    return "바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요";
  }
  return null;
}
