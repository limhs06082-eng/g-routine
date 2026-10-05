import { normalizeDataDir } from "./dataDir";

describe("normalizeDataDir", () => {
  it("드라이브 루트에는 G-routine\\data를 붙인다", () => {
    expect(normalizeDataDir("D:\\")).toBe("D:\\G-routine\\data");
  });

  it("끝의 역슬래시를 떼고 G-routine\\data를 붙인다", () => {
    expect(normalizeDataDir("D:\\내 자료\\")).toBe("D:\\내 자료\\G-routine\\data");
  });

  it("이미 G-routine\\data로 끝나면 대소문자와 상관없이 그대로 둔다", () => {
    expect(normalizeDataDir("d:\\g-routine\\DATA")).toBe("d:\\g-routine\\DATA");
    expect(normalizeDataDir("D:\\G-routine\\data\\")).toBe("D:\\G-routine\\data");
  });

  it("이름 일부만 같은 폴더는 맞추지 않는다", () => {
    expect(normalizeDataDir("D:\\myG-routine\\data")).toBe("D:\\myG-routine\\data\\G-routine\\data");
  });
});
