import React from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import "pretendard/dist/web/variable/pretendardvariable.css";
import "./index.css";
import { currentWindowLabel } from "@/lib/windowLabel";
import { WidgetApp } from "@/widget/WidgetApp";
import { ManagerApp } from "@/manager/ManagerApp";

async function start() {
  if (import.meta.env.DEV && !isTauri()) {
    const { installMockBackend } = await import("@/dev/mockBackend");
    installMockBackend();
    document.documentElement.dataset.preview = "true";
  }
  const label = currentWindowLabel();
  document.documentElement.dataset.window = label;
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>{label === "manager" ? <ManagerApp /> : <WidgetApp />}</React.StrictMode>,
  );
}

void start();
