"use client"

import * as React from "react"
import { Minus, Square, X } from "lucide-react"

import { Button } from "@/components/ui/button"

type WindowControlsProps = {
  onMinimize?: () => void
  onMaximize?: () => void
  onClose?: () => void
}

export function WindowControls({ onMinimize, onMaximize, onClose }: WindowControlsProps) {
  const [platform, setPlatform] = React.useState<"mac" | "windows" | "linux" | "unknown">("unknown")

  React.useEffect(() => {
    // Detect platform
    const userAgent = window.navigator.userAgent.toLowerCase()
    if (userAgent.indexOf("mac") !== -1) {
      setPlatform("mac")
    } else if (userAgent.indexOf("win") !== -1) {
      setPlatform("windows")
    } else if (userAgent.indexOf("linux") !== -1) {
      setPlatform("linux")
    }
  }, [])

  if (platform === "mac") {
    return (
      <div className="flex items-center gap-1.5 px-2 pl-3">
        <button
          onClick={onClose}
          className="group flex h-3 w-3 items-center justify-center rounded-full bg-red-500 transition-colors hover:bg-red-600"
          title="Close"
        >
          <X className="invisible h-2 w-2 text-red-900 group-hover:visible" />
          <span className="sr-only">Close</span>
        </button>
        <button
          onClick={onMinimize}
          className="group flex h-3 w-3 items-center justify-center rounded-full bg-yellow-500 transition-colors hover:bg-yellow-600"
          title="Minimize"
        >
          <Minus className="invisible h-2 w-2 text-yellow-900 group-hover:visible" />
          <span className="sr-only">Minimize</span>
        </button>
        <button
          onClick={onMaximize}
          className="group flex h-3 w-3 items-center justify-center rounded-full bg-green-500 transition-colors hover:bg-green-600"
          title="Maximize"
        >
          <Square className="invisible h-2 w-2 text-green-900 group-hover:visible" />
          <span className="sr-only">Maximize</span>
        </button>
      </div>
    )
  }

  // Windows/Linux/Default style - icons always visible
  return (
    <div className="flex items-center">
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 rounded-none hover:bg-muted"
        onClick={onMinimize}
        title="Minimize"
      >
        <Minus className="h-4 w-4" />
        <span className="sr-only">Minimize</span>
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 rounded-none hover:bg-muted"
        onClick={onMaximize}
        title="Maximize"
      >
        <Square className="h-4 w-4" />
        <span className="sr-only">Maximize</span>
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 rounded-none hover:bg-destructive hover:text-destructive-foreground"
        onClick={onClose}
        title="Close"
      >
        <X className="h-4 w-4" />
        <span className="sr-only">Close</span>
      </Button>
    </div>
  )
}

