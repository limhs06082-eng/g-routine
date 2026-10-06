import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppStatus, Settings, TodayView } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn(() => Promise.resolve("0.3.0")) }));

const apiMock = vi.hoisted(() => ({
  status: vi.fn(),
  settings: vi.fn(),
  setSetting: vi.fn(),
  today: vi.fn(),
  resizeWidget: vi.fn(() => Promise.resolve(false)),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { WidgetApp } from "./WidgetApp";

const status: AppStatus = { ready: true, corrupt: false, openFailed: false, portable: false, previousDir: null, suggestedDir: "", dataDir: "D:\G-routine\data", shortcut: true };
const settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: true, dayStartHour: 4, theme: "lavender", dueAlerts: true, hideHolidays: true, miniMode: false, vacationStart: null, vacationEnd: null, seenVersion: null };
const view: TodayView = { day: "2026-10-12", rest: null, pending: [], done: [] };

beforeEach(() => {
  vi.clearAllMocks();
  apiMock.status.mockResolvedValue(status);
  apiMock.today.mockResolvedValue(view);
});

test("after an update the widget shows what changed once, until confirmed", async () => {
  const user = userEvent.setup();
  apiMock.settings.mockResolvedValue(settings);
  apiMock.setSetting.mockResolvedValue({ ...settings, seenVersion: "0.3.0" });
  render(<WidgetApp />);
  const card = await screen.findByRole("region", { name: "새 버전에서 바뀐 점" });
  expect(card).toHaveTextContent("v0.3.0에서 바뀐 점");
  await user.click(screen.getByRole("button", { name: "확인" }));
  expect(apiMock.setSetting).toHaveBeenCalledWith("seen_version", "0.3.0");
  await waitFor(() => expect(screen.queryByRole("region", { name: "새 버전에서 바뀐 점" })).not.toBeInTheDocument());
});

test("nothing is shown when this version was already seen", async () => {
  apiMock.settings.mockResolvedValue({ ...settings, seenVersion: "0.3.0" });
  render(<WidgetApp />);
  expect(await screen.findByText("오늘의 루틴")).toBeInTheDocument();
  expect(screen.queryByRole("region", { name: "새 버전에서 바뀐 점" })).not.toBeInTheDocument();
  expect(apiMock.setSetting).not.toHaveBeenCalled();
});

test("mini mode shows only the pill", async () => {
  apiMock.settings.mockResolvedValue({ ...settings, miniMode: true, seenVersion: "0.3.0" });
  render(<WidgetApp />);
  expect(await screen.findByRole("button", { name: /크게 보기/ })).toBeInTheDocument();
  expect(screen.queryByText("오늘의 루틴")).not.toBeInTheDocument();
});
