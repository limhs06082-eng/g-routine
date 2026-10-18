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
  // 기간을 빠르게 바꾸는 등으로 응답이 순서가 뒤바뀌어 와도 마지막 요청의 응답만 쓴다
  const seq = useRef(0);

  const reload = useCallback(async () => {
    const mine = ++seq.current;
    try {
      const value = await loadRef.current();
      if (mine !== seq.current) return;
      setData(value);
      setError(null);
    } catch (e) {
      if (mine !== seq.current) return;
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

const THEME_KEY = "g-routine-theme";

function rememberTheme(theme: string) {
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // 저장소를 못 써도 다음 실행에 잠깐 기본 색이 보일 뿐이다
  }
}

/** 지난번 테마를 시작하자마자 칠한다 (설정을 불러오는 동안 라벤더가 잠깐 보이지 않게) */
export function applyRememberedTheme() {
  try {
    const theme = localStorage.getItem(THEME_KEY);
    document.documentElement.dataset.theme = theme ?? "lavender";
  } catch {
    document.documentElement.dataset.theme = "lavender";
  }
}

export function useSettings(enabled: boolean) {
  const { data, setData } = useData<Settings | null>(
    () => (enabled ? api.settings() : Promise.resolve(null)),
    [enabled],
  );
  const theme = data?.theme ?? null;
  useEffect(() => {
    // 설정을 불러오기 전에는 바꾸지 않는다 (main.tsx가 기억해 둔 테마를 먼저 칠해 둔다)
    if (!theme) return;
    document.documentElement.dataset.theme = theme;
    rememberTheme(theme);
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
/** 위젯 보통 너비(논리 px). Rust shell::window::WIDGET_WIDTH와 같다 */
export const WIDGET_WIDTH = 280;

export function useAutoResize<T extends HTMLElement>(normalWidth: number | null = null) {
  const ref = useRef<T | null>(null);
  const [capped, setCapped] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    let last = "";
    let watchedPill: Element | null = null;
    let frame = 0;
    const report = () => {
      frame = 0;
      const pill = el.querySelector("[data-mini-pill]");
      if (pill && pill !== watchedPill) {
        watchedPill = pill;
        ro.observe(pill);
      }
      const h = Math.ceil(el.getBoundingClientRect().height);
      // 알약 너비 + 바깥 여백(p-1.5 양쪽 12px). 아니면 보통 너비(글자를 크게 보면 그만큼 넓게)
      const w = pill ? Math.ceil(pill.getBoundingClientRect().width) + 12 : normalWidth;
      const key = `${h}x${w}`;
      if (key !== last) {
        last = key;
        api
          .resizeWidget(h, w)
          .then((c) => setCapped(Boolean(c)))
          .catch(() => {});
      }
    };
    const ro = new ResizeObserver(() => {
      if (!frame) frame = requestAnimationFrame(report);
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      if (frame) cancelAnimationFrame(frame);
    };
  }, [normalWidth]);
  return { ref, capped };
}
