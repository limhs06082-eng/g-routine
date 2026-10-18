import { act, render } from "@testing-library/react";

const apiMock = vi.hoisted(() => ({ resizeWidget: vi.fn(() => Promise.resolve(false)) }));
vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { useAutoResize } from "./hooks";

let observed: (() => void) | null = null;
beforeEach(() => {
  vi.clearAllMocks();
  observed = null;
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(cb: () => void) {
        observed = cb;
      }
      observe() {}
      disconnect() {}
    },
  );
  // 한 화면 그리기를 기다리지 않고 바로 돌린다
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    cb(0);
    return 1;
  });
  vi.stubGlobal("cancelAnimationFrame", () => {});
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const w = this.dataset.miniPill !== undefined ? 88 : 280;
    return { width: w, height: 120, top: 0, left: 0, right: w, bottom: 120, x: 0, y: 0, toJSON: () => ({}) } as DOMRect;
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function Probe({ width, pill }: { width: number | null; pill?: boolean }) {
  const { ref } = useAutoResize<HTMLDivElement>(width);
  return <div ref={ref}>{pill && <div data-mini-pill />}</div>;
}

test("the widget reports its height and, with large text, its wider width", () => {
  const { rerender } = render(<Probe width={null} />);
  act(() => observed?.());
  expect(apiMock.resizeWidget).toHaveBeenLastCalledWith(120, null);
  // 같은 크기면 다시 보내지 않는다
  act(() => observed?.());
  expect(apiMock.resizeWidget).toHaveBeenCalledTimes(1);
  // 글자를 크게 하면 넓어진 너비를 다시 알린다
  rerender(<Probe width={322} />);
  act(() => observed?.());
  expect(apiMock.resizeWidget).toHaveBeenLastCalledWith(120, 322);
});

test("in mini mode the window shrinks to the pill plus its margin", () => {
  render(<Probe width={null} pill />);
  act(() => observed?.());
  expect(apiMock.resizeWidget).toHaveBeenLastCalledWith(120, 88 + 12);
});
