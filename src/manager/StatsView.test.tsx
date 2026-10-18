import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Stats } from "@/lib/api";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
const dialogMock = vi.hoisted(() => ({ save: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialogMock);
const apiMock = vi.hoisted(() => ({ historyStats: vi.fn(), exportCsv: vi.fn() }));
vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { buildBars, percent, StatsView } from "./StatsView";

test("bars are daily up to a month and weekly beyond", () => {
  const days = [{ day: "2026-10-12", total: 4, completed: 3 }];
  const daily = buildBars("2026-10-10", "2026-10-16", days);
  expect(daily).toHaveLength(7);
  expect(daily[2]).toMatchObject({ label: "10/12", total: 4, completed: 3 });
  const weekly = buildBars("2026-07-20", "2026-10-17", days);
  expect(weekly.length).toBeLessThanOrEqual(14);
  expect(weekly.find((b) => b.key === "2026-10-12")).toMatchObject({ label: "10/12주", total: 4, completed: 3 });
  expect(percent(0, 0)).toBe(0);
});

test("stats show the rate, streak and most-missed routines, and save CSV", async () => {
  const user = userEvent.setup();
  apiMock.historyStats.mockImplementation((from: string, to: string): Promise<Stats> =>
    Promise.resolve({
      from,
      to,
      days: [{ day: to, total: 4, completed: 3 }],
      total: 4,
      completed: 3,
      streak: 5,
      routines: [
        { routineId: 2, title: "공문 확인", total: 2, completed: 1 },
        { routineId: 1, title: "출결 확인", total: 2, completed: 2 },
      ],
    }),
  );
  dialogMock.save.mockResolvedValue("C:/t/기록.csv");
  apiMock.exportCsv.mockResolvedValue(undefined);
  render(<StatsView />);
  expect(await screen.findByText("75%")).toBeInTheDocument();
  expect(screen.getByText("5일")).toBeInTheDocument();
  const list = screen.getByRole("region", { name: "루틴별 달성률" });
  expect(list.textContent?.indexOf("공문 확인")).toBeLessThan(list.textContent?.indexOf("출결 확인") ?? 0);

  await user.click(screen.getByRole("radio", { name: "최근 7일" }));
  const calls = apiMock.historyStats.mock.calls;
  const [from, to] = calls[calls.length - 1] as [string, string];
  expect(to).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  expect(from < to).toBe(true);

  await user.click(screen.getByRole("button", { name: "CSV로 저장 (엑셀)" }));
  expect(apiMock.exportCsv).toHaveBeenCalledWith("C:/t/기록.csv", from, to);
  expect(await screen.findByText("CSV 파일로 저장했어요. 엑셀에서 열 수 있어요")).toBeInTheDocument();
});
