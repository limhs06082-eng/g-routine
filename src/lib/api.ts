import { invoke } from "@tauri-apps/api/core";

export type RepeatType = "daily" | "weekdays" | "once" | "monthly";
/** 하루 중 언제 하는 일인지: 조회 전 · 수업 중 · 방과 후 (없으면 언제든) */
export type Slot = "morning" | "class" | "after";
export type ThemeName = "lavender" | "mint" | "peach" | "sky" | "lemon";
export type SettingKey = "always_on_top" | "autostart" | "hide_weekends" | "day_start_hour" | "theme" | "due_alerts" | "hide_holidays" | "mini_mode" | "seen_version" | "widget_scale" | "widget_opacity";
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
  /** 매월 반복의 날짜 (1–31, 0은 마지막 평일) */
  monthDay: number | null;
  /** 기간 한정 (YYYY-MM-DD, 둘 다 포함) */
  activeFrom: string | null;
  activeUntil: string | null;
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
  monthDay: number | null;
  activeFrom: string | null;
  activeUntil: string | null;
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
  /** 위젯 글자 크기 */
  widgetScale: "normal" | "large";
  /** 위젯 불투명도(%): 100 · 90 · 80 */
  widgetOpacity: number;
}

/** 공휴일 표가 어느 해까지 있는지. endingSoon: 11월부터 내년 표가 아직 없음, missing: 올해 표가 없음 */
export interface HolidayCoverage {
  lastYear: number | null;
  state: "ok" | "endingSoon" | "missing";
}

/** 두 번째 백업(OneDrive · USB, 주 1회) */
export interface Backup2Info {
  /** 고른 폴더 (그 안의 'G-routine 백업' 폴더에 쓴다) */
  dir: string | null;
  /** 마지막으로 백업한 날 (YYYY-MM-DD) */
  last: string | null;
  /** 아직 정하지 않았을 때 먼저 권할 OneDrive 폴더 */
  suggested: string | null;
  /** 위젯의 권유 카드를 닫았는지 (폴더를 한 번 정했어도 다시 묻지 않는다) */
  dismissed: boolean;
  /** 고른 폴더가 알맞지 않을 때의 안내 (C드라이브, 기록과 같은 드라이브) */
  warning: string | null;
  /** 마지막 자동 백업이 실패했으면 그 안내 */
  error: string | null;
}

export interface RoutineStat {
  routineId: number;
  title: string;
  total: number;
  completed: number;
}

/** 기간 통계 */
export interface Stats {
  from: string;
  to: string;
  /** 기록이 있는 날만, 날짜순 */
  days: DaySummary[];
  total: number;
  completed: number;
  /** 달성률이 낮은 루틴부터 */
  routines: RoutineStat[];
  /** 오늘까지 모두 마친 날이 이어진 수 */
  streak: number;
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
  /** 주기 확인에서 찾았지만 아직 설치하지 않은 새 버전. 위젯이 '지금 설치'를 보여 준다 */
  pendingUpdate: string | null;
  /** 컴퓨터 켜면 자동 시작 등록(또는 해제)이 실패했는지 (학교 보안 정책 등) */
  autostartFailed: boolean;
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
  historyStats: (from: string, to: string) => invoke<Stats>("history_stats", { from, to }),
  /** 기간의 기록을 CSV(엑셀)로 저장한다 */
  exportCsv: (path: string, from: string, to: string) => invoke<void>("export_csv", { path, from, to }),
  settings: () => invoke<Settings>("get_settings"),
  setSetting: (key: SettingKey, value: string) => invoke<Settings>("set_setting", { key, value }),
  /** 방학 기간을 정하거나(시작일·끝나는 날) 둘 다 null로 해제한다 */
  setVacation: (start: string | null, end: string | null) => invoke<Settings>("set_vacation", { start, end }),
  /** 문제 보고용 정보(상태 · 설정 · 데이터 개수 · 최근 로그)를 글 파일로 저장한다 */
  saveDiagnostics: (path: string) => invoke<void>("save_diagnostics", { path }),
  backup2Info: () => invoke<Backup2Info>("backup2_info"),
  /** 두 번째 백업 폴더를 정하거나(곧바로 한 번 백업한다) null로 끈다 */
  setBackup2Dir: (dir: string | null) => invoke<Backup2Info>("set_backup2_dir", { dir }),
  backup2Now: () => invoke<Backup2Info>("backup2_now"),
  dismissBackup2: () => invoke<void>("dismiss_backup2"),
  holidayCoverage: () => invoke<HolidayCoverage>("holiday_coverage"),
  openLink: (routineId: number) => invoke<void>("open_link", { routineId }),
  exportBackup: (path: string) => invoke<void>("export_backup", { path }),
  importBackup: (path: string) => invoke<void>("import_backup", { path }),
  changeDataDir: (dir: string) => invoke<void>("change_data_dir", { dir }),
  openManager: () => invoke<void>("open_manager"),
  /** 위젯 높이를 내용에 맞춘다. true면 화면 높이 상한에 걸려 스크롤이 필요하다. */
  resizeWidget: (height: number, width: number | null = null) => invoke<boolean>("resize_widget", { height, width }),
  hideWidget: () => invoke<void>("hide_widget"),
  /** 준비된 새 버전을 받아서 설치한다. 설치가 시작되면 앱이 꺼졌다 다시 켜진다 */
  installUpdate: () => invoke<void>("install_update"),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "알 수 없는 오류가 생겼어요. 다시 시도해 주세요";
}
