import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { DayItem } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const apiMock = vi.hoisted(() => ({
  historyMonth: vi.fn(),
  historyDay: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { HistoryTab } from "./HistoryTab";

const item = (id: number, title: string, completedAt: string | null): DayItem => ({
  id,
  day: "2026-10-05",
  routineId: id + 10,
  title,
  sortOrder: id,
  completedAt,
  repeatType: "daily",
  dueTime: null,
  hasLink: false,
  overdue: false,
  slot: null,
});

const items = [item(1, "출결 확인", "2026-10-05T09:12:00"), item(2, "가정통신문 회수", null)];

beforeEach(() => {
  vi.clearAllMocks();
  apiMock.historyMonth.mockResolvedValue([]);
  apiMock.historyDay.mockResolvedValue(items);
});

test("lists day items with completion time or pending label", async () => {
  render(<HistoryTab />);
  expect(await screen.findByText("출결 확인")).toBeInTheDocument();
  expect(screen.getByText("09:12")).toBeInTheDocument();
  expect(screen.getByText("가정통신문 회수")).toBeInTheDocument();
  expect(screen.getByText("미완료")).toBeInTheDocument();
  expect(screen.getByText("2개 중 1개 완료")).toBeInTheDocument();
});

test("a failed day load shows an error with retry instead of the empty state", async () => {
  apiMock.historyDay.mockRejectedValueOnce(new Error("boom"));
  render(<HistoryTab />);
  expect(await screen.findByText("기록을 불러오지 못했어요.")).toBeInTheDocument();
  expect(screen.queryByText("이날은 기록이 없어요.")).not.toBeInTheDocument();

  await userEvent.click(screen.getByRole("button", { name: "다시 시도" }));
  expect(await screen.findByText("출결 확인")).toBeInTheDocument();
  expect(screen.queryByText("기록을 불러오지 못했어요.")).not.toBeInTheDocument();
});
