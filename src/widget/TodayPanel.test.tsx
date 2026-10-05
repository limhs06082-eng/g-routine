import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { DayItem, TodayView } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const apiMock = vi.hoisted(() => ({
  today: vi.fn(),
  setDone: vi.fn(),
  quickAdd: vi.fn(),
  openLink: vi.fn(),
  openManager: vi.fn(),
  hideWidget: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { TodayPanel } from "./TodayPanel";

const item = (id: number, title: string, extra: Partial<DayItem> = {}): DayItem => ({
  id,
  day: "2026-10-05",
  routineId: id + 10,
  title,
  sortOrder: id,
  completedAt: null,
  repeatType: "daily",
  dueTime: null,
  hasLink: false,
  ...extra,
});

const view: TodayView = {
  day: "2026-10-05",
  weekendHidden: false,
  pending: [item(1, "출결 확인", { hasLink: true, dueTime: "09:00" }), item(2, "가정통신문 회수", { repeatType: "once" })],
  done: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  apiMock.today.mockResolvedValue(JSON.parse(JSON.stringify(view)));
  apiMock.setDone.mockResolvedValue(undefined);
  apiMock.quickAdd.mockResolvedValue(undefined);
  apiMock.openLink.mockResolvedValue(undefined);
});

const renderPanel = () => render(<TodayPanel pinned onTogglePin={() => Promise.resolve()} />);

test("shows date, progress and chips", async () => {
  renderPanel();
  expect(await screen.findByText("10월 5일 월요일")).toBeInTheDocument();
  expect(screen.getByTestId("progress-count")).toHaveTextContent("0 / 2");
  expect(screen.getByText("오늘만")).toBeInTheDocument();
  expect(screen.getByText("09:00")).toBeInTheDocument();
});

test("checking hides the item and undo brings it back", async () => {
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 완료" }));
  await waitFor(() => expect(screen.queryByRole("button", { name: "출결 확인 완료" })).not.toBeInTheDocument());
  expect(apiMock.setDone).toHaveBeenCalledWith(1, true);
  expect(screen.getByTestId("progress-count")).toHaveTextContent("1 / 2");

  await user.click(screen.getByRole("button", { name: "되돌리기" }));
  expect(await screen.findByRole("button", { name: "출결 확인 완료" })).toBeInTheDocument();
  expect(apiMock.setDone).toHaveBeenLastCalledWith(1, false);
});

test("failed save keeps the item, clears the fade and allows a retry", async () => {
  apiMock.setDone.mockRejectedValueOnce("데이터를 저장하거나 읽지 못했어요");
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 완료" }));
  expect(await screen.findByText("데이터를 저장하거나 읽지 못했어요")).toBeInTheDocument();
  const button = screen.getByRole("button", { name: "출결 확인 완료" });
  await waitFor(() => expect(button.closest("li")).not.toHaveClass("opacity-0"));
  expect(apiMock.setDone).toHaveBeenCalledTimes(1);

  await user.click(button);
  await waitFor(() => expect(apiMock.setDone).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.queryByRole("button", { name: "출결 확인 완료" })).not.toBeInTheDocument());
});

test("failed save reloads the list so a day rollover is picked up", async () => {
  apiMock.setDone.mockRejectedValueOnce("날짜가 바뀌었어요. 목록을 새로 불러올게요");
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 완료" }));
  expect(await screen.findByText("날짜가 바뀌었어요. 목록을 새로 불러올게요")).toBeInTheDocument();
  await waitFor(() => expect(apiMock.today).toHaveBeenCalledTimes(2));
});

test("load failure shows a retry button", async () => {
  apiMock.today.mockRejectedValueOnce("x");
  const user = userEvent.setup();
  renderPanel();
  expect(await screen.findByText("오늘 목록을 불러오지 못했어요.")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "다시 시도" }));
  expect(await screen.findByText("10월 5일 월요일")).toBeInTheDocument();
  expect(screen.queryByText("오늘 목록을 불러오지 못했어요.")).not.toBeInTheDocument();
});

test("failed pin toggle shows the error", async () => {
  const user = userEvent.setup();
  const onTogglePin = vi.fn().mockRejectedValueOnce("고정하지 못했어요");
  render(<TodayPanel pinned onTogglePin={onTogglePin} />);
  await user.click(await screen.findByRole("button", { name: "맨 위 고정 해제" }));
  expect(await screen.findByText("고정하지 못했어요")).toBeInTheDocument();
});

test("quick add sends trimmed text on Enter", async () => {
  const user = userEvent.setup();
  renderPanel();
  const input = await screen.findByLabelText("오늘 할 일 추가");
  await user.type(input, "  학부모 상담 전화  {Enter}");
  expect(apiMock.quickAdd).toHaveBeenCalledWith("학부모 상담 전화");
  await waitFor(() => expect(input).toHaveValue(""));
});

test("link button opens the routine link", async () => {
  const user = userEvent.setup();
  renderPanel();
  await user.click(await screen.findByRole("button", { name: "출결 확인 바로가기 열기" }));
  expect(apiMock.openLink).toHaveBeenCalledWith(11);
});

test("all done shows a finished message", async () => {
  apiMock.today.mockResolvedValue({ ...view, pending: [], done: [item(1, "출결 확인", { completedAt: "2026-10-05T08:47:00" })] });
  renderPanel();
  expect(await screen.findByText("오늘 루틴을 모두 마쳤어요")).toBeInTheDocument();
  expect(screen.getByTestId("progress-count")).toHaveTextContent("모두 완료");
});
