"use client"
import { Button } from "@/components/ui/button"
import { MovingBorder } from "@/components/ui/moving-border-wrapper"
import { cn } from "@/lib/utils"
import type { ButtonProps } from "@/components/ui/button"

interface MovingBorderButtonProps extends ButtonProps {
  borderColor?: "orange" | "blue" | "green" | "purple" | "red"
  wrapperClassName?: string
}

export function MovingBorderButton({
  children,
  className,
  borderColor = "orange",
  wrapperClassName,
  ...props
}: MovingBorderButtonProps) {
  return (
    <MovingBorder borderColor={borderColor} className={cn("rounded-md", wrapperClassName)}>
      <Button className={cn("relative z-10", className)} {...props}>
        {children}
      </Button>
    </MovingBorder>
  )
}
