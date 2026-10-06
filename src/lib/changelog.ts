/**
 * 업데이트 뒤 위젯에 한 번 보여 주는 '바뀐 점'. 새 버전을 릴리스할 때 맨 위에 추가한다.
 * 위젯 폭이 좁으니 한 줄에 짧게 쓴다.
 */
export const CHANGELOG: { version: string; items: string[] }[] = [
  {
    version: "0.3.0",
    items: [
      "Ctrl+Alt+G로 위젯을 숨기고 다시 띄워요 (수업 화면 띄울 때)",
      "⤡ 버튼으로 작게 보기: 알약 모양 ✓ 3/7",
      "⚙ 관리 창에서 루틴마다 시간대를 정하면 조회 전 · 수업 중 · 방과 후로 묶여 보여요",
      "요일 지정에 토 · 일이 생겼어요",
      "공휴일 정보가 인터넷으로 자동 갱신돼요",
    ],
  },
];

/** 이 기록이 생기기 전 버전. 안내를 본 기록이 없으면 이 버전에서 올라온 것으로 본다. */
const BEFORE_CHANGELOG = "0.2.0";

function parts(v: string): number[] {
  return v.split(".").map((p) => Number(p) || 0);
}

export function compareVersions(a: string, b: string): number {
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++) {
    if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) - (y[i] ?? 0);
  }
  return 0;
}

/** 아직 보지 않은 바뀐 점 (새 버전부터). seen이 없으면 v0.2.0 이하에서 올라온 것이다. */
export function unseenNotes(seen: string | null, current: string) {
  const from = seen ?? BEFORE_CHANGELOG;
  return CHANGELOG.filter((e) => compareVersions(e.version, from) > 0 && compareVersions(e.version, current) <= 0);
}
