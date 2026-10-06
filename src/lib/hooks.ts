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

/**
 * 위젯 내용 높이를 Rust에 알려 창 높이를 맞춘다 (아래 모서리 고정).
 * 미니 모드에서는 알약(data-mini-pill)의 너비도 알려 창을 알약 크기로 줄인다 (오른쪽 모서리 고정).
 * 투명한 부분도 창이라 클릭을 막으므로, 보이는 만큼만 창이어야 한다.
 * `capped`는 내용이 화면 높이 상한을 넘었다는 뜻이고, 이때만 스크롤을 켠다.
 */
export function useAutoResize<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);
  const [capped, setCapped] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    let last = "";
    let watchedPill: Element | null = null;
    const ro = new ResizeObserver(() => {
      const pill = el.querySelector("[data-mini-pill]");
      if (pill && pill !== watchedPill) {
        watchedPill = pill;
        ro.observe(pill);
      }
      const h = Math.ceil(el.getBoundingClientRect().height);
      // 알약 너비 + 바깥 여백(p-1.5 양쪽 12px)
      const w = pill ? Math.ceil(pill.getBoundingClientRect().width) + 12 : null;
      const key = `${h}x${w}`;
      if (key !== last) {
        last = key;
        api
          .resizeWidget(h, w)
          .then((c) => setCapped(Boolean(c)))
          .catch(() => {});
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return { ref, capped };
}
