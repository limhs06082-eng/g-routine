import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowLabel = "widget" | "manager";

export function currentWindowLabel(): WindowLabel {
  let label = new URLSearchParams(window.location.search).get("window") ?? "widget";
  try {
    label = getCurrentWindow().label;
  } catch {
    // 브라우저 미리보기: 쿼리 값 사용
  }
  return label === "manager" ? "manager" : "widget";
}
