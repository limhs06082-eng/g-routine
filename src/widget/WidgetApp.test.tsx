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
  backup2Info: vi.fn(),
  setBackup2Dir: vi.fn(),
  dismissBackup2: vi.fn(),
  installUpdate: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { WidgetApp } from "./WidgetApp";

const status: AppStatus = { ready: true, corrupt: false, openFailed: false, portable: false, previousDir: null, suggestedDir: "", dataDir: "D:\\G-routine\\data", shortcut: true, pendingUpdate: null, autostartFailed: false };
const settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: true, dayStartHour: 4, theme: "lavender", dueAlerts: true, hideHolidays: true, miniMode: false, vacationStart: null, vacationEnd: null, seenVersion: null };
const view: TodayView = { day: "2026-10-12", rest: null, pending: [], done: [] };

beforeEach(() => {
  vi.clearAllMocks();
  apiMock.status.mockResolvedValue(status);
  apiMock.today.mockResolvedValue(view);
  apiMock.backup2Info.mockResolvedValue({ dir: null, last: null, suggested: null, dismissed: false });
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

test("a pending update is offered with an install button", async () => {
  const user = userEvent.setup();
  apiMock.status.mockResolvedValue({ ...status, pendingUpdate: "0.4.0" });
  apiMock.settings.mockResolvedValue({ ...settings, seenVersion: "0.3.0" });
  apiMock.installUpdate.mockReturnValue(new Promise(() => {}));
  render(<WidgetApp />);
  const card = await screen.findByRole("region", { name: "새 버전 안내" });
  expect(card).toHaveTextContent("v0.4.0");
  await user.click(screen.getByRole("button", { name: "지금 설치" }));
  expect(apiMock.installUpdate).toHaveBeenCalledTimes(1);
  expect(await screen.findByText("설치하는 중…")).toBeInTheDocument();
});

test("a failed install shows the error and lets the user try again", async () => {
  const user = userEvent.setup();
  apiMock.status.mockResolvedValue({ ...status, pendingUpdate: "0.4.0" });
  apiMock.settings.mockResolvedValue({ ...settings, seenVersion: "0.3.0" });
  apiMock.installUpdate.mockRejectedValue("새 버전을 설치하지 못했어요");
  render(<WidgetApp />);
  await user.click(await screen.findByRole("button", { name: "지금 설치" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("새 버전을 설치하지 못했어요");
  expect(screen.getByRole("button", { name: "지금 설치" })).toBeEnabled();
});

test("mini mode shows only the pill", async () => {
  apiMock.settings.mockResolvedValue({ ...settings, miniMode: true, seenVersion: "0.3.0" });
  render(<WidgetApp />);
  expect(await screen.findByRole("button", { name: /크게 보기/ })).toBeInTheDocument();
  expect(screen.queryByText("오늘의 루틴")).not.toBeInTheDocument();
});

test("a PC with OneDrive is offered a weekly backup once", async () => {
  const user = userEvent.setup();
  apiMock.settings.mockResolvedValue({ ...settings, seenVersion: "9.9.9" });
  apiMock.backup2Info.mockResolvedValueOnce({ dir: null, last: null, suggested: "C:/Users/t/OneDrive", dismissed: false });
  apiMock.setBackup2Dir.mockResolvedValue({ dir: "C:/Users/t/OneDrive", last: "2026-10-12", suggested: null, dismissed: false });
  render(<WidgetApp />);
  const card = await screen.findByRole("region", { name: "OneDrive 백업 권유" });
  expect(card).toHaveTextContent("OneDrive에 백업해 둘까요?");
  await user.click(screen.getByRole("button", { name: "백업하기" }));
  expect(apiMock.setBackup2Dir).toHaveBeenCalledWith("C:/Users/t/OneDrive");
  await waitFor(() => expect(screen.queryByRole("region", { name: "OneDrive 백업 권유" })).not.toBeInTheDocument());
});

test("the backup offer stays away once dismissed or already set up", async () => {
  apiMock.settings.mockResolvedValue({ ...settings, seenVersion: "9.9.9" });
  apiMock.backup2Info.mockResolvedValue({ dir: null, last: null, suggested: "C:/Users/t/OneDrive", dismissed: true });
  render(<WidgetApp />);
  expect(await screen.findByText("오늘의 루틴")).toBeInTheDocument();
  expect(screen.queryByRole("region", { name: "OneDrive 백업 권유" })).not.toBeInTheDocument();
});
