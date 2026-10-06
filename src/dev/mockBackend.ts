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
    sortOrder: id,
    createdAt: `${TODAY}T08:00:00`,
    archivedAt: null,
    ...extra,
  };
}

const routines: Routine[] = [
  routine(1, "출결 확인", { dueTime: "09:00", link: "https://www.neis.go.kr" }),
  routine(2, "수업 준비"),
  routine(3, "누가기록 작성", { link: "https://www.neis.go.kr" }),
  routine(4, "공문 확인", { link: "https://www.google.com" }),
  routine(5, "알림장 작성"),
  routine(6, "주간학습안내 배부", { repeatType: "weekdays", weekdays: 16 }),
  routine(7, "가정통신문 회수", { repeatType: "once", onceDate: TODAY }),
];
const doneAt = new Map<number, string>([[2, `${TODAY}T08:55:00`]]);
let settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: false, dayStartHour: 4, theme: "lavender", dueAlerts: true };
let ready = new URLSearchParams(window.location.search).get("setup") !== "1";
const DATA_DIR = "D:\\G-routine\\data";

function bit(day: string) {
  const [y, m, d] = day.split("-").map(Number);
  const wd = new Date(y, m - 1, d).getDay(); // 0=일
  return 1 << ((wd + 6) % 7);
}

function applies(r: Routine, day: string) {
  if (r.archivedAt) return false;
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
  };
}

function today(): TodayView {
  const items = routines
    .filter((r) => applies(r, TODAY))
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((r) => toItem(r, TODAY, doneAt.get(r.id) ?? null));
  return {
    day: TODAY,
    weekendHidden: false,
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
  return { ready, corrupt: false, openFailed: false, portable: false, previousDir: null, suggestedDir: DATA_DIR, dataDir: ready ? DATA_DIR : null };
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
};

export function installMockBackend() {
  const label = new URLSearchParams(window.location.search).get("window") ?? "widget";
  mockWindows(label);
  mockIPC((cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    if (cmd.startsWith("plugin:event|")) return 0;
    if (cmd.startsWith("plugin:dialog|")) return null;
    if (cmd === "plugin:app|version") return "0.1.0";
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
        const value = field === "dayStartHour" ? Number(raw) : field === "theme" ? raw : raw === "true";
        settings = { ...settings, [field]: value } as Settings;
        changed();
        return settings;
      }
      case "open_link":
        console.info("[mock] open_link", args.routineId);
        return null;
      case "open_manager":
        window.open("?window=manager", "_blank");
        return null;
      default:
        return null;
    }
  });
}
