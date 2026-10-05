import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppStatus } from "@/lib/api";

const dialogMock = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialogMock);

const apiMock = vi.hoisted(() => ({
  setup: vi.fn(),
  restoreBackup: vi.fn(),
  retryBoot: vi.fn(),
  inspectDataDir: vi.fn(),
}));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { Setup } from "./Setup";

const base: AppStatus = {
  ready: false,
  corrupt: false,
  openFailed: false,
  portable: false,
  previousDir: null,
  suggestedDir: "D:\\G-routine\\data",
  dataDir: null,
};

beforeEach(() => {
  vi.clearAllMocks();
});

test("open failure shows a retry button instead of the restore box", async () => {
  apiMock.retryBoot.mockResolvedValue(undefined);
  render(<Setup status={{ ...base, openFailed: true, previousDir: "D:\\G-routine\\data" }} />);

  expect(screen.getByText(/데이터 파일을 열 수 없어요\. 다른 프로그램이 사용 중일 수 있어요/)).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "최근 백업으로 복구" })).not.toBeInTheDocument();
  expect(screen.queryByText(/예전 저장 폴더/)).not.toBeInTheDocument();

  await userEvent.click(screen.getByRole("button", { name: "다시 시도" }));
  expect(apiMock.retryBoot).toHaveBeenCalledTimes(1);
});

test("corrupt status offers restore and explains starting fresh", () => {
  render(<Setup status={{ ...base, corrupt: true, previousDir: "D:\\G-routine\\data" }} />);
  expect(screen.getByRole("button", { name: "최근 백업으로 복구" })).toBeInTheDocument();
  expect(
    screen.getByText("복구가 안 되면 아래에서 시작하기를 누르세요. 손상된 파일은 따로 보관돼요."),
  ).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "다시 시도" })).not.toBeInTheDocument();
});

test("picking a folder shows the normalized G-routine\\data path and sends it to setup", async () => {
  dialogMock.open.mockResolvedValue("E:\\내 자료\\");
  apiMock.inspectDataDir.mockRejectedValue("no backend");
  apiMock.setup.mockResolvedValue(undefined);
  render(<Setup status={base} />);

  await userEvent.click(screen.getByRole("button", { name: "저장 폴더 고르기" }));
  await waitFor(() => expect(screen.getByText("E:\\내 자료\\G-routine\\data")).toBeInTheDocument());

  await userEvent.click(screen.getByRole("button", { name: "시작하기" }));
  expect(apiMock.setup).toHaveBeenCalledWith("E:\\내 자료\\G-routine\\data", "homeroom");
});

test("the app's inspection result wins over the local guess", async () => {
  dialogMock.open.mockResolvedValue("E:\\old");
  apiMock.inspectDataDir.mockResolvedValue({ normalized: "E:\\old", hasData: true });
  render(<Setup status={base} />);

  await userEvent.click(screen.getByRole("button", { name: "저장 폴더 고르기" }));
  await waitFor(() => expect(screen.getByText("E:\\old")).toBeInTheDocument());
  expect(apiMock.inspectDataDir).toHaveBeenCalledWith("E:\\old");
});
