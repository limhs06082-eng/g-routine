import { invoke } from "@tauri-apps/api/core";

export type RepeatType = "daily" | "weekdays" | "once";
export type ThemeName = "lavender" | "mint" | "peach" | "sky" | "lemon";
export type SettingKey = "always_on_top" | "autostart" | "hide_weekends" | "day_start_hour" | "theme";
export type TemplateName = "homeroom" | "subject" | "empty";

export interface Routine {
  id: number;
  title: string;
  repeatType: RepeatType;
  weekdays: number;
  onceDate: string | null;
  dueTime: string | null;
  link: string | null;
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
}

export interface TodayView {
  day: string;
  weekendHidden: boolean;
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
  openLink: (routineId: number) => invoke<void>("open_link", { routineId }),
  exportBackup: (path: string) => invoke<void>("export_backup", { path }),
  importBackup: (path: string) => invoke<void>("import_backup", { path }),
  changeDataDir: (dir: string) => invoke<void>("change_data_dir", { dir }),
  openManager: () => invoke<void>("open_manager"),
  resizeWidget: (height: number) => invoke<void>("resize_widget", { height }),
  hideWidget: () => invoke<void>("hide_widget"),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "알 수 없는 오류가 생겼어요. 다시 시도해 주세요";
}
