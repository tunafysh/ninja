"use client"
 
import * as React from "react"
import { MoonIcon, SunIcon } from "lucide-react"
import { useTheme } from "next-themes"
 
import { Button } from "@/components/ui/button"

export function ModeToggle() {
  const { theme, setTheme } = useTheme()

  function switchTheme() {
    if (theme === "dark") {
      setTheme("light")
    } else {
      setTheme("dark")
    }
  }

  return (
    <Button variant={"toggle"} size="icon" onClick={switchTheme}>
        <SunIcon className={`h-[1.2rem] w-[1.2rem] rotate-0 transition-all dark:-rotate-90 ${theme == "light"? "opacity-100": "opacity-0"}`} />
        <MoonIcon className={`absolute h-[1.2rem] w-[1.2rem] rotate-90 transition-all dark:rotate-0 ${theme == "dark"? "opacity-100": "opacity-0"}`} />
        <span className="sr-only">Toggle theme</span>
    </Button>
  )
}