import * as React from "react";
import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "h-9 w-full min-w-0 rounded-lg border border-input bg-background px-3 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-soft-3 focus-visible:ring-2 focus-visible:ring-soft disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
