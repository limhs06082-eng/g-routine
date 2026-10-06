import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppStatus, Settings } from "@/lib/api";

const dialogMock = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialogMock);

const apiMock = vi.hoisted(() => ({
  inspectDataDir: vi.fn(),
  changeDataDir: vi.fn(),
  exportBackup: vi.fn(),
  importBackup: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { SettingsTab } from "./SettingsTab";

const settings: Settings = { alwaysOnTop: true, autostart: true, hideWeekends: false, dayStartHour: 4, theme: "lavender", dueAlerts: true };
const status: AppStatus = {
  ready: true,
  corrupt: false,
  openFailed: false,
  portable: false,
  previousDir: null,
  suggestedDir: "D:\\G-routine\\data",
  dataDir: "D:\\G-routine\\data",
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
