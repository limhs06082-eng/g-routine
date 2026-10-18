import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppStatus, Settings } from "@/lib/api";

const dialogMock = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialogMock);

const apiMock = vi.hoisted(() => ({
  inspectDataDir: vi.fn(),
  changeDataDir: vi.fn(),
  exportBackup: vi.fn(),
  importBackup: vi.fn(),
  setVacation: vi.fn(),
  saveDiagnostics: vi.fn(),
  backup2Info: vi.fn(() => Promise.resolve({ dir: null as string | null, last: null as string | null, suggested: null as string | null, dismissed: false })),
  setBackup2Dir: vi.fn(),
  backup2Now: vi.fn(),
  holidayCoverage: vi.fn(() => Promise.resolve({ lastYear: 2027, state: "ok" })),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { holidayHint, SettingsTab } from "./SettingsTab";
import { vacationLabel } from "./VacationSetting";

const settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: false, dayStartHour: 4, theme: "lavender", dueAlerts: true, hideHolidays: true, miniMode: false, vacationStart: null, vacationEnd: null, seenVersion: null, widgetScale: "normal", widgetOpacity: 100 };
const status: AppStatus = {
  ready: true,
  corrupt: false,
  openFailed: false,
  portable: false,
  previousDir: null,
  suggestedDir: "D:\\G-routine\\data",
  dataDir: "D:\\G-routine\\data",
  shortcut: true,
  pendingUpdate: null,
  autostartFailed: false,
};

function renderTab() {
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  return screen.getByRole("button", { name: "변경" });
}

beforeEach(() => {
  vi.clearAllMocks();
});

test("changing to a folder without data moves there and shows the normalized path", async () => {
  dialogMock.open.mockResolvedValue("E:\\");
  apiMock.inspectDataDir.mockResolvedValue({ normalized: "E:\\G-routine\\data", hasData: false });
  apiMock.changeDataDir.mockResolvedValue(undefined);
  await userEvent.click(renderTab());

  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("저장 위치를 바꿨어요"));
  expect(dialogMock.confirm).not.toHaveBeenCalled();
  expect(apiMock.changeDataDir).toHaveBeenCalledWith("E:\\G-routine\\data");
  expect(screen.getByText("E:\\G-routine\\data")).toBeInTheDocument();
});

test("a folder that already has data asks first, and cancelling stops quietly", async () => {
  dialogMock.open.mockResolvedValue("E:\\G-routine\\data");
  apiMock.inspectDataDir.mockResolvedValue({ normalized: "E:\\G-routine\\data", hasData: true });
  dialogMock.confirm.mockResolvedValue(false);
  await userEvent.click(renderTab());

  await waitFor(() => expect(dialogMock.confirm).toHaveBeenCalledTimes(1));
  expect(dialogMock.confirm.mock.calls[0][0]).toBe(
    "선택한 폴더에 이미 G-routine 데이터가 있어요. 지금 데이터 대신 그 데이터를 사용할까요? 지금 데이터는 원래 폴더에 그대로 남아요.",
  );
  expect(dialogMock.confirm.mock.calls[0][1]).toEqual({ title: "저장 위치 변경", kind: "warning" });
  expect(apiMock.changeDataDir).not.toHaveBeenCalled();
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

test("confirming switches to the existing data", async () => {
  dialogMock.open.mockResolvedValue("E:\\G-routine\\data");
  apiMock.inspectDataDir.mockResolvedValue({ normalized: "E:\\G-routine\\data", hasData: true });
  dialogMock.confirm.mockResolvedValue(true);
  apiMock.changeDataDir.mockResolvedValue(undefined);
  await userEvent.click(renderTab());

  await waitFor(() => expect(apiMock.changeDataDir).toHaveBeenCalledWith("E:\\G-routine\\data"));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("저장 위치를 바꿨어요"));
});

test("day start hour offers every hour and keeps an afternoon value selected", () => {
  render(<SettingsTab settings={{ ...settings, dayStartHour: 15 }} status={status} onChange={vi.fn()} />);
  const select = screen.getByLabelText("하루 시작 시각") as HTMLSelectElement;
  expect(select.options).toHaveLength(24);
  expect(select.options[23]).toHaveTextContent("오후 11시");
  expect(select.value).toBe("15");
  expect(select.selectedOptions[0]).toHaveTextContent("오후 3시");
});

test("the due alert switch turns alerts off", async () => {
  const onChange = vi.fn().mockResolvedValue(settings);
  render(<SettingsTab settings={settings} status={status} onChange={onChange} />);
  const toggle = screen.getByRole("switch", { name: "마감 시각 알림" });
  expect(toggle).toBeChecked();
  await userEvent.click(toggle);
  expect(onChange).toHaveBeenCalledWith("due_alerts", "false");
});

test("the holiday switch turns holiday hiding off", async () => {
  const onChange = vi.fn().mockResolvedValue(settings);
  render(<SettingsTab settings={settings} status={status} onChange={onChange} />);
  await userEvent.click(screen.getByRole("switch", { name: "공휴일에는 숨기기" }));
  expect(onChange).toHaveBeenCalledWith("hide_holidays", "false");
});

test("saving a vacation sends both dates", async () => {
  apiMock.setVacation.mockResolvedValue({ ...settings, vacationStart: "2026-12-24", vacationEnd: "2027-02-28" });
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  fireEvent.change(screen.getByLabelText("방학 시작일"), { target: { value: "2026-12-24" } });
  fireEvent.change(screen.getByLabelText("방학 끝나는 날"), { target: { value: "2027-02-28" } });
  await userEvent.click(screen.getByRole("button", { name: "방학 저장" }));
  expect(apiMock.setVacation).toHaveBeenCalledWith("2026-12-24", "2027-02-28");
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("방학 기간을 저장했어요"));
});

test("a vacation ending before it starts is rejected before saving", async () => {
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  fireEvent.change(screen.getByLabelText("방학 시작일"), { target: { value: "2026-12-24" } });
  fireEvent.change(screen.getByLabelText("방학 끝나는 날"), { target: { value: "2026-12-01" } });
  await userEvent.click(screen.getByRole("button", { name: "방학 저장" }));
  expect(screen.getByRole("alert")).toHaveTextContent("시작일이 끝나는 날보다 늦어요");
  expect(apiMock.setVacation).not.toHaveBeenCalled();
});

test("an active vacation can be cleared", async () => {
  apiMock.setVacation.mockResolvedValue(settings);
  const onVacation = { ...settings, vacationStart: "2026-12-24", vacationEnd: "2027-02-28" };
  render(<SettingsTab settings={onVacation} status={status} onChange={vi.fn()} />);
  expect(screen.getByText("2026년 12월 24일 ~ 2027년 2월 28일 동안 반복 루틴을 쉬어요")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "방학 해제" }));
  expect(apiMock.setVacation).toHaveBeenCalledWith(null, null);
});

test("a vacation within one year is shown without the year", () => {
  expect(vacationLabel("2027-07-20", "2027-08-23")).toBe("7월 20일 ~ 8월 23일 동안 반복 루틴을 쉬어요");
});

test("the shortcut row explains when Ctrl+Alt+G is taken by another program", () => {
  render(<SettingsTab settings={settings} status={{ ...status, shortcut: false }} onChange={vi.fn()} />);
  expect(screen.getByText(/다른 프로그램이 이 단축키를 쓰고 있어서 쓸 수 없어요/)).toBeInTheDocument();
});

test("mini mode can be switched on from settings", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn().mockResolvedValue({ ...settings, miniMode: true });
  render(<SettingsTab settings={settings} status={status} onChange={onChange} />);
  await user.click(screen.getByRole("switch", { name: "작게 보기" }));
  expect(onChange).toHaveBeenCalledWith("mini_mode", "true");
});

test("the holiday row says how far the table goes and warns when it runs out", async () => {
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  expect(await screen.findByText(/2027년까지 들어 있고/)).toBeInTheDocument();
  expect(holidayHint({ lastYear: 2027, state: "endingSoon" })).toBe("2028년 공휴일 정보가 아직 없어요. 발표되면 인터넷으로 자동으로 받아요");
  expect(holidayHint({ lastYear: 2027, state: "missing" })).toMatch(/올해 공휴일 정보가 아직 없어서/);
  expect(holidayHint(null)).toBe("설날 · 추석 · 대체공휴일 등");
});

test("diagnostics are saved where the teacher picks", async () => {
  const user = userEvent.setup();
  dialogMock.save.mockResolvedValue("C:\\Users\\t\\보고.txt");
  apiMock.saveDiagnostics.mockResolvedValue(undefined);
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "문제 보고용 정보 저장" }));
  expect(apiMock.saveDiagnostics).toHaveBeenCalledWith("C:\\Users\\t\\보고.txt");
  expect(await screen.findByText("문제 보고용 정보를 저장했어요. 이 파일을 보내 주세요")).toBeInTheDocument();
});

test("a failed autostart registration is explained", () => {
  render(<SettingsTab settings={settings} status={{ ...status, autostartFailed: true }} onChange={vi.fn()} />);
  expect(screen.getByText(/자동 시작을 등록하지 못했어요/)).toBeInTheDocument();
});

test("settings offer OneDrive first for the second backup", async () => {
  const user = userEvent.setup();
  apiMock.backup2Info.mockResolvedValueOnce({ dir: null, last: null, suggested: "C:/Users/t/OneDrive", dismissed: false });
  apiMock.setBackup2Dir.mockResolvedValue({ dir: "C:/Users/t/OneDrive", last: "2026-10-12", suggested: null, dismissed: false });
  render(<SettingsTab settings={settings} status={status} onChange={vi.fn()} />);
  await user.click(await screen.findByRole("button", { name: "OneDrive에 백업하기" }));
  expect(apiMock.setBackup2Dir).toHaveBeenCalledWith("C:/Users/t/OneDrive");
  expect(await screen.findByText(/폴더에 일주일마다 백업해요/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "지금 백업" })).toBeInTheDocument();
});

test("text size and opacity are chosen from settings", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn().mockResolvedValue(settings);
  render(<SettingsTab settings={settings} status={status} onChange={onChange} />);
  await user.click(screen.getByRole("radio", { name: "크게" }));
  expect(onChange).toHaveBeenCalledWith("widget_scale", "large");
  await user.click(screen.getByRole("radio", { name: "80%" }));
  expect(onChange).toHaveBeenCalledWith("widget_opacity", "80");
});
