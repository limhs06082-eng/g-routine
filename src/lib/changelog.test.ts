import { CHANGELOG, compareVersions, unseenNotes } from "./changelog";

test("versions compare numerically", () => {
  expect(compareVersions("0.10.0", "0.9.9")).toBeGreaterThan(0);
  expect(compareVersions("0.3.0", "0.3.0")).toBe(0);
  expect(compareVersions("0.2.9", "0.3.0")).toBeLessThan(0);
});

test("notes newer than what was seen, up to the running version", () => {
  // 안내를 본 기록이 없으면 v0.2.0 이하에서 올라온 것
  expect(unseenNotes(null, "0.3.0").map((n) => n.version)).toEqual(["0.3.0"]);
  expect(unseenNotes("0.3.0", "0.3.0")).toEqual([]);
  // 아직 설치되지 않은 버전의 안내는 보이지 않는다
  expect(unseenNotes("0.2.0", "0.2.5")).toEqual([]);
});

test("the newest changelog entry comes first and has short lines", () => {
  const versions = CHANGELOG.map((e) => e.version);
  expect([...versions].sort((a, b) => compareVersions(b, a))).toEqual(versions);
  for (const e of CHANGELOG) for (const item of e.items) expect([...item].length).toBeLessThanOrEqual(60);
});
