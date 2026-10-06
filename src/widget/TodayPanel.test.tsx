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
  overdue: false,
  ...extra,
});

const view: TodayView = {
  day: "2026-10-05",
  rest: null,
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

const noop = () => Promise.resolve();
const renderPanel = () => render(<TodayPanel pinned onTogglePin={noop} mini={false} onToggleMini={noop} />);

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
  render(<TodayPanel pinned onTogglePin={onTogglePin} mini={false} onToggleMini={noop} />);
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

test("long lists render every item without an inner scroll area", async () => {
  const many = Array.from({ length: 15 }, (_, i) => item(i + 1, `루틴 ${i + 1}`));
  apiMock.today.mockResolvedValue({ ...view, pending: many });
  renderPanel();
  expect(await screen.findByText("루틴 15")).toBeInTheDocument();
  const list = screen.getByTestId("pending-list");
  expect(list.className).not.toMatch(/overflow|max-h/);
  expect(list.querySelectorAll("li")).toHaveLength(15);
});

test("an item past its due time shows a red due chip", async () => {
  apiMock.today.mockResolvedValue({
    ...view,
    pending: [item(1, "출결 확인", { dueTime: "09:00", overdue: true }), item(2, "공문 확인", { dueTime: "15:00" })],
  });
  renderPanel();
  const late = await screen.findByTitle("마감 09:00 지남");
  expect(late).toHaveTextContent("09:00");
  expect(late.className).toMatch(/text-danger/);
  expect(screen.getByText("15:00").className).not.toMatch(/text-danger/);
});

test.each([
  [{ kind: "holiday", name: "한글날" }, "오늘은 한글날이에요. 푹 쉬세요"],
  [{ kind: "holiday", name: "추석 연휴" }, "오늘은 추석 연휴예요. 푹 쉬세요"],
  [{ kind: "holiday", name: "대체공휴일(개천절)" }, "오늘은 대체공휴일(개천절)이에요. 푹 쉬세요"],
  [{ kind: "vacation", name: "방학" }, "방학 중이에요. 푹 쉬세요"],
  [{ kind: "weekend", name: "주말" }, "좋은 주말 보내세요"],
] as const)("a rest day with nothing to do says why (%o)", async (rest, message) => {
  apiMock.today.mockResolvedValue({ ...view, rest, pending: [], done: [] });
  renderPanel();
  expect(await screen.findByText(message)).toBeInTheDocument();
});

test("mini mode shows only a pill with progress and overdue count", async () => {
  const user = userEvent.setup();
  const onToggleMini = vi.fn().mockResolvedValue(undefined);
  apiMock.today.mockResolvedValue({
    ...view,
    pending: [item(1, "출결 확인", { dueTime: "09:00", overdue: true }), item(2, "공문 확인")],
    done: [item(3, "수업 준비", { completedAt: "2026-10-05T08:30:00" })],
  });
  render(<TodayPanel pinned onTogglePin={noop} mini onToggleMini={onToggleMini} />);
  const pill = await screen.findByRole("button", { name: "크게 보기 · 오늘 할 일 3개 중 1개 완료 · 마감 지난 일 1개" });
  expect(pill).toHaveTextContent("1/3");
  expect(screen.queryByText("출결 확인")).not.toBeInTheDocument();
  await user.click(pill);
  expect(onToggleMini).toHaveBeenCalledTimes(1);
});

test("the header can shrink the widget into mini mode", async () => {
  const user = userEvent.setup();
  const onToggleMini = vi.fn().mockResolvedValue(undefined);
  render(<TodayPanel pinned onTogglePin={noop} mini={false} onToggleMini={onToggleMini} />);
  await user.click(await screen.findByRole("button", { name: "작게 보기" }));
  expect(onToggleMini).toHaveBeenCalledTimes(1);
});
