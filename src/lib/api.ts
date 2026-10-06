import { invoke } from "@tauri-apps/api/core";

export type RepeatType = "daily" | "weekdays" | "once";
/** 하루 중 언제 하는 일인지: 조회 전 · 수업 중 · 방과 후 (없으면 언제든) */
export type Slot = "morning" | "class" | "after";
export type ThemeName = "lavender" | "mint" | "peach" | "sky" | "lemon";
export type SettingKey = "always_on_top" | "autostart" | "hide_weekends" | "day_start_hour" | "theme" | "due_alerts" | "hide_holidays" | "mini_mode" | "seen_version";
export type TemplateName = "homeroom" | "subject" | "empty";

export interface Routine {
  id: number;
  title: string;
  repeatType: RepeatType;
  weekdays: number;
  onceDate: string | null;
  dueTime: string | null;
  link: string | null;
  slot: Slot | null;
  sortOrder: number;
  createdAt: string;
  archivedAt: string | null;
}

export interface RoutineInput {
  title: string;
  repeatType: RepeatType;
  weekdays: number;
  onceDate: string | null;
  dueTime: string | null;
  link: string | null;
  slot: Slot | null;
}

export interface DayItem {
  id: number;
  day: string;
  routineId: number;
  title: string;
  sortOrder: number;
  completedAt: string | null;
  repeatType: RepeatType;
  dueTime: string | null;
  hasLink: boolean;
  slot: Slot | null;
  /** 오늘 목록에서만: 끝내지 않았는데 마감 시각이 지났는지 */
  overdue: boolean;
}

export interface Rest {
  kind: "weekend" | "holiday" | "vacation";
  /** 화면에 보일 이름 (예: "추석", "방학", "주말") */
  name: string;
}

export interface TodayView {
  day: string;
  /** 오늘이 쉬는 날(공휴일·방학·주말)이면 그 이유. 쉬는 날에는 반복 루틴이 숨겨진다 */
  rest: Rest | null;
  pending: DayItem[];
  done: DayItem[];
}

export interface DaySummary {
  day: string;
  total: number;
  completed: number;
}

export interface Settings {
  alwaysOnTop: boolean;
  autostart: boolean;
  hideWeekends: boolean;
  dayStartHour: number;
  theme: ThemeName;
  dueAlerts: boolean;
  hideHolidays: boolean;
  /** 위젯을 알약 모양(✓ 3/7)으로 작게 보여 줄지 */
  miniMode: boolean;
  /** 방학 · 쉬는 기간 (YYYY-MM-DD, 둘 다 있거나 둘 다 없다) */
  vacationStart: string | null;
  vacationEnd: string | null;
  /** '바뀐 점' 안내를 마지막으로 본 버전 (없으면 v0.2.0 이하에서 올라온 것) */
  seenVersion: string | null;
}

/** 공휴일 표가 어느 해까지 있는지. endingSoon: 11월부터 내년 표가 아직 없음, missing: 올해 표가 없음 */
export interface HolidayCoverage {
  lastYear: number | null;
  state: "ok" | "endingSoon" | "missing";
}

export interface AppStatus {
  ready: boolean;
  corrupt: boolean;
  /** DB 파일이 있지만 지금은 열 수 없음 (다른 프로그램이 사용 중 등). 손상과 다르다. */
  openFailed: boolean;
  portable: boolean;
  previousDir: string | null;
  suggestedDir: string;
  dataDir: string | null;
  /** 전역 단축키(Ctrl+Alt+G)를 등록했는지. 다른 프로그램이 쓰고 있으면 false */
  shortcut: boolean;
}

export interface DataDirInfo {
  /** 실제로 쓰일 폴더 (…\G-routine\data 모양, 이미 데이터가 있는 폴더면 그대로) */
  normalized: string;
  hasData: boolean;
}

export const api = {
  status: () => invoke<AppStatus>("get_status"),
  setup: (dir: string, template: TemplateName) => invoke<void>("setup", { dir, template }),
  restoreBackup: () => invoke<void>("restore_backup"),
  retryBoot: () => invoke<void>("retry_boot"),
  inspectDataDir: (dir: string) => invoke<DataDirInfo>("inspect_data_dir", { dir }),
  today: () => invoke<TodayView>("get_today"),
  setDone: (itemId: number, done: boolean) => invoke<void>("set_done", { itemId, done }),
  quickAdd: (title: string) => invoke<void>("quick_add", { title }),
  listRoutines: () => invoke<Routine[]>("list_routines"),
  createRoutine: (input: RoutineInput) => invoke<number>("create_routine", { input }),
  updateRoutine: (id: number, input: RoutineInput) => invoke<void>("update_routine", { id, input }),
  archiveRoutine: (id: number) => invoke<void>("archive_routine", { id }),
  reorderRoutines: (ids: number[]) => invoke<void>("reorder_routines", { ids }),
  historyMonth: (year: number, month: number) => invoke<DaySummary[]>("history_month", { year, month }),
  historyDay: (day: string) => invoke<DayItem[]>("history_day", { day }),
  settings: () => invoke<Settings>("get_settings"),
  setSetting: (key: SettingKey, value: string) => invoke<Settings>("set_setting", { key, value }),
  /** 방학 기간을 정하거나(시작일·끝나는 날) 둘 다 null로 해제한다 */
  setVacation: (start: string | null, end: string | null) => invoke<Settings>("set_vacation", { start, end }),
  holidayCoverage: () => invoke<HolidayCoverage>("holiday_coverage"),
  openLink: (routineId: number) => invoke<void>("open_link", { routineId }),
  exportBackup: (path: string) => invoke<void>("export_backup", { path }),
  importBackup: (path: string) => invoke<void>("import_backup", { path }),
  changeDataDir: (dir: string) => invoke<void>("change_data_dir", { dir }),
  openManager: () => invoke<void>("open_manager"),
  /** 위젯 높이를 내용에 맞춘다. true면 화면 높이 상한에 걸려 스크롤이 필요하다. */
  resizeWidget: (height: number, width: number | null = null) => invoke<boolean>("resize_widget", { height, width }),
  hideWidget: () => invoke<void>("hide_widget"),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "알 수 없는 오류가 생겼어요. 다시 시도해 주세요";
}
