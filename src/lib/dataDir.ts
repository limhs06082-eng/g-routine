/**
 * 고른 폴더를 자동 탐색되는 모양(…\G-routine\data)으로 맞춘다. 화면 표시용이다.
 * 이미 데이터가 있는 폴더는 그대로 쓰는 판단은 프런트에서 할 수 없으므로 Rust(normalize_data_dir)가 최종 결정한다.
 */
export function normalizeDataDir(picked: string): string {
  const trimmed = picked.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/);
  const [parent, last] = parts.slice(-2).map((p) => p.toLowerCase());
  if (parts.length >= 2 && parent === "g-routine" && last === "data") return trimmed;
  return `${trimmed}\\G-routine\\data`;
}
