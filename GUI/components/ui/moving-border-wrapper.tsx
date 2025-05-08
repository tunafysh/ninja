"use client"
import type React from "react"
import { motion, useAnimationFrame, useMotionTemplate, useMotionValue, useTransform } from "motion/react"
import { useRef } from "react"
import { cn } from "@/lib/utils"

export const MovingBorder = ({
  children,
  duration = 3000,
  rx = "30%",
  ry = "30%",
  borderColor = "orange",
  className,
  ...otherProps
}: {
  children: React.ReactNode
  duration?: number
  rx?: string
  ry?: string
  borderColor?: "orange" | "blue" | "green" | "purple" | "red"
  className?: string
  [key: string]: any
}) => {
  const pathRef = useRef<any>(null)
  const progress = useMotionValue<number>(0)

  useAnimationFrame((time: number) => {
    const length = pathRef.current?.getTotalLength()
    if (length) {
      const pxPerMillisecond = length / duration
      progress.set((time * pxPerMillisecond) % length)
    }
  })

  const x = useTransform(progress, (val) => pathRef.current?.getPointAtLength(val).x)
  const y = useTransform(progress, (val) => pathRef.current?.getPointAtLength(val).y)

  const transform = useMotionTemplate`translateX(${x}px) translateY(${y}px) translateX(-50%) translateY(-50%)`

  const colorMap = {
    orange: "bg-[radial-gradient(#f97316_40%,transparent_60%)]",
    blue: "bg-[radial-gradient(#0ea5e9_40%,transparent_60%)]",
    green: "bg-[radial-gradient(#22c55e_40%,transparent_60%)]",
    purple: "bg-[radial-gradient(#a855f7_40%,transparent_60%)]",
    red: "bg-[radial-gradient(#ef4444_40%,transparent_60%)]",
  }

  return (
    <div className={cn("relative p-[1px] overflow-hidden group", className)} {...otherProps}>
      <div className="absolute inset-0">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          preserveAspectRatio="none"
          className="absolute h-full w-full"
          width="100%"
          height="100%"
        >
          <rect fill="none" width="100%" height="100%" rx={rx} ry={ry} ref={pathRef} />
        </svg>
        <motion.div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            display: "inline-block",
            transform,
          }}
        >
          <div className={cn("h-20 w-20 opacity-[0.8]", colorMap[borderColor])} />
        </motion.div>
      </div>
      <div className="relative z-10">{children}</div>
    </div>
  )
}
