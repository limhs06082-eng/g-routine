import { cn } from "@/lib/utils";

test("cn merges tailwind classes and drops falsy values", () => {
  expect(cn("px-2", false && "hidden", "px-4")).toBe("px-4");
});
