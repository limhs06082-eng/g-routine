import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg text-sm font-medium whitespace-nowrap transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-strong",
        outline: "border border-input bg-background hover:bg-muted",
        ghost: "text-foreground hover:bg-soft",
        soft: "bg-soft text-ink hover:bg-soft-2",
        destructive: "bg-danger text-white hover:opacity-90",
      },
      size: {
        default: "h-9 px-4",
        sm: "h-8 px-3 text-xs",
        icon: "size-8",
        "icon-sm": "size-6 rounded-md",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

function Button({
  className,
  variant,
  size,
  type = "button",
  ...props
}: React.ComponentProps<"button"> & VariantProps<typeof buttonVariants>) {
  return <button type={type} data-slot="button" className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}

export { Button, buttonVariants };
