import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorMessage, type SettingKey, type Settings } from "@/lib/api";

export const DATA_CHANGED = "data-changed";

/** 불러오기 + data-changed 이벤트 · 창 focus 때 자동 새로고침 */
export function useData<T>(load: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const loadRef = useRef(load);
  loadRef.current = load;

  const reload = useCallback(async () => {
    try {
      setData(await loadRef.current());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reload, ...deps]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen(DATA_CHANGED, () => void reload())
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    const onFocus = () => void reload();
    window.addEventListener("focus", onFocus);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("focus", onFocus);
    };
  }, [reload]);

  return { data, error, reload, setData };
}

export function useSettings(enabled: boolean) {
  const { data, setData } = useData<Settings | null>(
    () => (enabled ? api.settings() : Promise.resolve(null)),
    [enabled],
  );
  const theme = data?.theme ?? "lavender";
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  const update = useCallback(
    async (key: SettingKey, value: string) => {
      const next = await api.setSetting(key, value);
      setData(next);
      return next;
    },
    [setData],
  );
  return { settings: data, update };
}

/** 위젯 내용 높이를 Rust에 알려 창 높이를 맞춘다 (아래 모서리 고정). */
export function useAutoResize<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    let last = 0;
    const ro = new ResizeObserver(() => {
      const h = Math.ceil(el.getBoundingClientRect().height);
      if (h !== last) {
        last = h;
        api.resizeWidget(h).catch(() => {});
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return ref;
}
