import { render } from "@testing-library/react";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const apiMock = vi.hoisted(() => ({ status: vi.fn(() => new Promise(() => {})) }));

vi.mock("@/lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/api")>()),
  api: apiMock,
}));

import { ManagerApp } from "./ManagerApp";

function fire(type: string) {
  const e = new Event(type, { bubbles: true, cancelable: true });
  document.body.dispatchEvent(e);
  return e.defaultPrevented;
}

test("file drops on the manager window are blocked while mounted and released on unmount", () => {
  const { unmount } = render(<ManagerApp />);
  expect(fire("dragover")).toBe(true);
  expect(fire("drop")).toBe(true);
  unmount();
  expect(fire("dragover")).toBe(false);
  expect(fire("drop")).toBe(false);
});
