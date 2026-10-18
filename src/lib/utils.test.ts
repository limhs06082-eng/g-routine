import { cn } from "@/lib/utils";

test("cn merges tailwind classes and drops falsy values", () => {
  const hidden = false as boolean;
  expect(cn("px-2", hidden && "hidden", "px-4")).toBe("px-4");
});
