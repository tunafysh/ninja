"use client"

import * as React from "react"
import { parseDate } from "chrono-node"
import { Clock } from "lucide-react"

import { Input } from "@/components/ui/input"

function toTimeOnly(date: Date) {
  const d = new Date(0)
  d.setHours(date.getHours())
  d.setMinutes(date.getMinutes())
  return d
}

function formatTime(date: Date | undefined) {
  if (!date) return ""
  return date.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false, // change to true if you want AM/PM
  })
}

export function TextTime({
  value,
  onChange,
  className,
}: {
  value: string
  onChange: (value: string) => void
  className?: string
}) {
  const [parsedTime, setParsedTime] = React.useState<Date | undefined>(undefined)

  function handleInput(text: string) {
    // always update input text
    onChange(text)

    const parsed = parseDate(text)

    if (parsed) {
      const timeOnly = toTimeOnly(parsed)
      setParsedTime(timeOnly)

      // AUTO-UPDATE INPUT WITH PARSED TIME
      const formatted = formatTime(timeOnly)
      onChange(formatted)
    }
  }

  return (
    <div className="relative">
      <Input
        value={value}
        placeholder='e.g. "half past 3", "15:30", "quarter to 5"'
        className={`pr-10 ${className}`}
        onChange={(e) => handleInput(e.target.value)}
      />

      <Clock className="absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 opacity-60" />
    </div>
  )
}
