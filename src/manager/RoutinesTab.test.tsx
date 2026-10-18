import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Routine } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const apiMock = vi.hoisted(() => ({
  listRoutines: vi.fn(),
  createRoutine: vi.fn(),
  updateRoutine: vi.fn(),
  archiveRoutine: vi.fn(),
  reorderRoutines: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { RoutinesTab } from "./RoutinesTab";

const routine = (id: number, title: string): Routine => ({
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
  createdAt: "",
  archivedAt: null,
});

beforeEach(() => {
  vi.clearAllMocks();
});

test("shows a retry button when loading fails, then renders routines", async () => {
  apiMock.listRoutines.mockRejectedValueOnce("boom").mockResolvedValue([routine(1, "출결 확인")]);
  const user = userEvent.setup();
  render(<RoutinesTab />);
  expect(await screen.findByText("루틴 목록을 불러오지 못했어요.")).toBeInTheDocument();
  expect(screen.queryByText(/아직 루틴이 없어요/)).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "다시 시도" }));
  expect(await screen.findByText("출결 확인")).toBeInTheDocument();
  expect(screen.queryByText("루틴 목록을 불러오지 못했어요.")).not.toBeInTheDocument();
});

test("shows the empty state only for a loaded empty list", async () => {
  apiMock.listRoutines.mockResolvedValue([]);
  render(<RoutinesTab />);
  expect(await screen.findByText(/아직 루틴이 없어요/)).toBeInTheDocument();
});

test("routines can be moved with the keyboard-friendly up and down buttons", async () => {
  const user = userEvent.setup();
  apiMock.listRoutines.mockResolvedValue([routine(1, "출결 확인"), routine(2, "수업 준비"), routine(3, "공문 확인")]);
  apiMock.reorderRoutines.mockResolvedValue(undefined);
  render(<RoutinesTab />);
  expect(await screen.findByRole("button", { name: "출결 확인 위로" })).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "공문 확인 위로" }));
  expect(apiMock.reorderRoutines).toHaveBeenCalledWith([1, 3, 2]);
  await user.click(screen.getByRole("button", { name: "출결 확인 아래로" }));
  expect(apiMock.reorderRoutines).toHaveBeenLastCalledWith([3, 1, 2]);
});
