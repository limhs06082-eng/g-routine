import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AppStatus, DayItem, DaySummary, Routine, RoutineInput, Settings, TodayView } from "@/lib/api";
import { normalizeDataDir } from "@/lib/dataDir";

const pad = (n: number) => String(n).padStart(2, "0");
const dayStr = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const stamp = (d: Date) => `${dayStr(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

const TODAY = dayStr(new Date());
let nextId = 100;

function routine(id: number, title: string, extra: Partial<Routine> = {}): Routine {
  return {
    id,
    title,
    repeatType: "daily",
    weekdays: 0,
    onceDate: null,
    dueTime: null,
    link: null,
    slot: null,
    monthDay: null,
    activeFrom: null,
    activeUntil: null,
    sortOrder: id,
    createdAt: `${TODAY}T08:00:00`,
    archivedAt: null,
    ...extra,
  };
}

const routines: Routine[] = [
  routine(1, "출결 확인", { dueTime: "09:00", link: "https://www.neis.go.kr", slot: "morning" }),
  routine(2, "수업 준비", { slot: "morning" }),
  routine(3, "누가기록 작성", { link: "https://www.neis.go.kr", slot: "after" }),
  routine(4, "공문 확인", { link: "https://www.google.com", slot: "after" }),
  routine(5, "알림장 작성", { slot: "after" }),
  routine(6, "주간학습안내 배부", { repeatType: "weekdays", weekdays: 16, slot: "after" }),
  routine(7, "가정통신문 회수", { repeatType: "once", onceDate: TODAY }),
];
const doneAt = new Map<number, string>([[2, `${TODAY}T08:55:00`]]);
let settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: false, dayStartHour: 4, theme: "lavender", dueAlerts: true, hideHolidays: true, miniMode: false, vacationStart: null, vacationEnd: null, seenVersion: null, widgetScale: "normal", widgetOpacity: 100 };
let ready = new URLSearchParams(window.location.search).get("setup") !== "1";
const DATA_DIR = "D:\\G-routine\\data";
let backup2 = { dir: null as string | null, last: null as string | null, suggested: "C:\\Users\\선생님\\OneDrive" as string | null, dismissed: false };

function bit(day: string) {
  const [y, m, d] = day.split("-").map(Number);
  const wd = new Date(y, m - 1, d).getDay(); // 0=일
  return 1 << ((wd + 6) % 7);
}

function applies(r: Routine, day: string) {
  if (r.archivedAt) return false;
  if ((r.activeFrom && day < r.activeFrom) || (r.activeUntil && day > r.activeUntil)) return false;
  if (r.repeatType === "monthly") {
    const [y, m, d] = day.split("-").map(Number);
    const last = new Date(y, m, 0).getDate();
    return r.monthDay === 0 ? d === last : r.monthDay === d;
  }
  if (r.repeatType === "daily") return true;
  if (r.repeatType === "weekdays") return (r.weekdays & bit(day)) !== 0;
  return r.onceDate === day;
}

function toItem(r: Routine, day: string, completedAt: string | null): DayItem {
  return {
    id: r.id,
    day,
    routineId: r.id,
    title: r.title,
    sortOrder: r.sortOrder,
    completedAt,
    repeatType: r.repeatType,
    dueTime: r.dueTime,
    hasLink: Boolean(r.link),
    overdue: false,
    slot: r.slot,
  };
}

function today(): TodayView {
  const items = routines
    .filter((r) => applies(r, TODAY))
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((r) => toItem(r, TODAY, doneAt.get(r.id) ?? null));
  return {
    day: TODAY,
    rest: null,
    pending: items.filter((i) => !i.completedAt),
    done: items.filter((i) => i.completedAt),
  };
}

function historyMonth(year: number, month: number): DaySummary[] {
  const out: DaySummary[] = [];
  const days = new Date(year, month, 0).getDate();
  for (let d = 1; d <= days; d++) {
    const day = `${year}-${pad(month)}-${pad(d)}`;
    if (day > TODAY) break;
    const wd = new Date(year, month - 1, d).getDay();
    if (wd === 0 || wd === 6) continue;
    if (day === TODAY) {
      const v = today();
      out.push({ day, total: v.pending.length + v.done.length, completed: v.done.length });
    } else {
      out.push({ day, total: 6, completed: [6, 4, 6, 2, 5, 0][d % 6] });
    }
  }
  return out;
}

function historyDay(day: string): DayItem[] {
  if (day === TODAY) {
    const v = today();
    return [...v.pending, ...v.done].sort((a, b) => a.sortOrder - b.sortOrder);
  }
  const summary = historyMonth(Number(day.slice(0, 4)), Number(day.slice(5, 7))).find((s) => s.day === day);
  if (!summary) return [];
  return routines
    .filter((r) => r.repeatType === "daily")
    .slice(0, summary.total)
    .map((r, i) => toItem(r, day, i < summary.completed ? `${day}T0${8 + Math.floor(i / 3)}:${pad(10 + i * 7)}:00` : null));
}

function status(): AppStatus {
  return { ready, corrupt: false, openFailed: false, portable: false, previousDir: null, suggestedDir: DATA_DIR, dataDir: ready ? DATA_DIR : null, shortcut: true, autostartFailed: false, pendingUpdate: new URLSearchParams(window.location.search).get("update") };
}

function changed() {
  setTimeout(() => window.dispatchEvent(new Event("focus")), 0);
}

function fromInput(id: number, input: RoutineInput, sortOrder: number): Routine {
  return routine(id, input.title, { ...input, sortOrder });
}

const KEY_MAP: Record<string, keyof Settings> = {
  always_on_top: "alwaysOnTop",
  autostart: "autostart",
  hide_weekends: "hideWeekends",
  day_start_hour: "dayStartHour",
  theme: "theme",
  due_alerts: "dueAlerts",
  hide_holidays: "hideHolidays",
  mini_mode: "miniMode",
  seen_version: "seenVersion",
  widget_scale: "widgetScale",
  widget_opacity: "widgetOpacity",
};

export function installMockBackend() {
  const label = new URLSearchParams(window.location.search).get("window") ?? "widget";
  mockWindows(label);
  mockIPC((cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    if (cmd.startsWith("plugin:event|")) return 0;
    if (cmd.startsWith("plugin:dialog|")) return null;
    if (cmd === "plugin:app|version") return "0.3.0";
    switch (cmd) {
      case "get_status":
        return status();
      case "setup":
        ready = true;
        changed();
        return null;
      case "retry_boot":
        changed();
        return null;
      case "inspect_data_dir":
        return { normalized: normalizeDataDir(String(args.dir)), hasData: false };
      case "get_today":
        return today();
      case "set_done": {
        const id = args.itemId as number;
        if (args.done) doneAt.set(id, stamp(new Date()));
        else doneAt.delete(id);
        changed();
        return null;
      }
      case "quick_add":
        routines.push(routine(nextId++, String(args.title), { repeatType: "once", onceDate: TODAY, sortOrder: 999 }));
        changed();
        return null;
      case "list_routines":
        return routines
          .filter((r) => !r.archivedAt && !(r.repeatType === "once" && (r.onceDate ?? "") < TODAY))
          .sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id);
      case "create_routine": {
        const id = nextId++;
        routines.push(fromInput(id, args.input as RoutineInput, routines.length));
        changed();
        return id;
      }
      case "update_routine": {
        const idx = routines.findIndex((r) => r.id === args.id);
        if (idx >= 0) routines[idx] = fromInput(routines[idx].id, args.input as RoutineInput, routines[idx].sortOrder);
        changed();
        return null;
      }
      case "archive_routine": {
        const r = routines.find((x) => x.id === args.id);
        if (r) r.archivedAt = stamp(new Date());
        changed();
        return null;
      }
      case "reorder_routines":
        (args.ids as number[]).forEach((id, i) => {
          const r = routines.find((x) => x.id === id);
          if (r) r.sortOrder = i;
        });
        changed();
        return null;
      case "history_month":
        return historyMonth(args.year as number, args.month as number);
      case "history_day":
        return historyDay(args.day as string);
      case "get_settings":
        return settings;
      case "set_setting": {
        const field = KEY_MAP[args.key as string];
        const raw = args.value as string;
        const value =
          field === "dayStartHour" || field === "widgetOpacity" ? Number(raw) : field === "theme" || field === "seenVersion" || field === "widgetScale" ? raw : raw === "true";
        settings = { ...settings, [field]: value } as Settings;
        changed();
        return settings;
      }
      case "backup2_info":
        return backup2;
      case "set_backup2_dir": {
        const dir = (args.dir as string | null) ?? null;
        backup2 = dir ? { ...backup2, dir, last: TODAY, suggested: null } : { ...backup2, dir: null, last: null, suggested: "C:\\Users\\선생님\\OneDrive" };
        changed();
        return backup2;
      }
      case "backup2_now":
        backup2 = { ...backup2, last: TODAY };
        return backup2;
      case "dismiss_backup2":
        backup2 = { ...backup2, dismissed: true };
        changed();
        return null;
      case "history_stats": {
        const from = args.from as string;
        const to = args.to as string;
        const days: DaySummary[] = [];
        for (let y = Number(from.slice(0, 4)), m = Number(from.slice(5, 7)); `${y}-${pad(m)}` <= to.slice(0, 7); m === 12 ? (y++, (m = 1)) : m++) {
          days.push(...historyMonth(y, m).filter((d) => d.day >= from && d.day <= to));
        }
        const total = days.reduce((s, d) => s + d.total, 0);
        const completed = days.reduce((s, d) => s + d.completed, 0);
        return {
          from,
          to,
          days,
          total,
          completed,
          streak: 3,
          routines: routines
            .filter((r) => r.repeatType === "daily")
            .map((r, i) => ({ routineId: r.id, title: r.title, total: days.length, completed: Math.max(0, days.length - i * 2) }))
            .sort((a, b) => a.completed / a.total - b.completed / b.total),
        };
      }
      case "export_csv":
        console.info("[mock] export_csv", args);
        return null;
      case "save_diagnostics":
        console.info("[mock] save_diagnostics", args.path);
        return null;
      case "holiday_coverage":
        return { lastYear: 2027, state: "ok" };
      case "set_vacation":
        settings = { ...settings, vacationStart: (args.start as string | null) ?? null, vacationEnd: (args.end as string | null) ?? null };
        changed();
        return settings;
      case "open_link":
        console.info("[mock] open_link", args.routineId);
        return null;
      case "open_manager":
        window.open("?window=manager", "_blank");
        return null;
      case "install_update":
        // 미리보기(?update=0.4.0): 잠깐 기다렸다가 실패해 오류 문구와 다시 시도를 볼 수 있게 한다
        return new Promise((_, reject) => setTimeout(() => reject("새 버전을 설치하지 못했어요. 잠시 후 다시 시도해 주세요 (미리보기)"), 1500));
      default:
        return null;
    }
  });
}
