"use client"

import { useRef, useState, useEffect } from "react"
import type { Terminal as XTerminal } from "xterm"
import type { FitAddon as XTermFitAddon } from "xterm-addon-fit"
import type { JSX } from "react/jsx-runtime"
import "xterm/css/xterm.css"

// Define proper types for the terminal themes
interface TerminalTheme {
  background: string
  foreground: string
  cursor: string
  selection: string
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

// Terminal themes with proper typing
const terminalThemes: Record<"dark" | "light", TerminalTheme> = {
  dark: {
    background: "#282c34",
    foreground: "#abb2bf",
    cursor: "#528bff",
    selection: "#3E4451",
    black: "#282c34",
    red: "#e06c75",
    green: "#98c379",
    yellow: "#e5c07b",
    blue: "#61afef",
    magenta: "#c678dd",
    cyan: "#56b6c2",
    white: "#abb2bf",
    brightBlack: "#5c6370",
    brightRed: "#e06c75",
    brightGreen: "#98c379",
    brightYellow: "#e5c07b",
    brightBlue: "#61afef",
    brightMagenta: "#c678dd",
    brightCyan: "#56b6c2",
    brightWhite: "#ffffff",
  },
  light: {
    background: "#ffffff",
    foreground: "#24292e",
    cursor: "#044289",
    selection: "#c8e1ff",
    black: "#24292e",
    red: "#d73a49",
    green: "#22863a",
    yellow: "#e36209",
    blue: "#005cc5",
    magenta: "#6f42c1",
    cyan: "#1b7c83",
    white: "#6a737d",
    brightBlack: "#959da5",
    brightRed: "#cb2431",
    brightGreen: "#28a745",
    brightYellow: "#f66a0a",
    brightBlue: "#2188ff",
    brightMagenta: "#8a63d2",
    brightCyan: "#3192aa",
    brightWhite: "#d1d5da",
  },
}

// Define props interface
interface TerminalComponentProps {
  isDarkMode: boolean
}

// Define window augmentation for global types
declare global {
  interface Window {
    Terminal?: typeof XTerminal
    FitAddon?: typeof XTermFitAddon
  }
}

// Command handler type
type CommandHandler = (args: string[], term: XTerminal) => void

// Available commands
const commands: Record<string, CommandHandler> = {
  clear: (_, term) => {
    term.clear()
  },
  help: (_, term) => {
    term.writeln("Available commands:")
    term.writeln("  clear - Clear the terminal screen")
    term.writeln("  help - Show this help message")
    term.writeln("  version - Show terminal version")
  },
  version: (_, term) => {
    term.writeln("Apache Configuration Terminal v1.0.0")
  },
}

export default function TerminalComponent({ isDarkMode }: TerminalComponentProps): JSX.Element {
  const terminalRef = useRef<HTMLDivElement>(null)
  const [isTerminalReady, setIsTerminalReady] = useState<boolean>(false)
  const [terminal, setTerminal] = useState<XTerminal | null>(null)
  const fitAddonRef = useRef<XTermFitAddon | null>(null)
  const commandBufferRef = useRef<string>("")

  // Load xterm scripts
  useEffect(() => {
    // Check if we're in the browser
    if (typeof window === "undefined") return

    // Load xterm dynamically
    const loadXterm = async (): Promise<void> => {
      try {
        // Import the modules
        const xtermModule = await import("xterm")
        const fitAddonModule = await import("xterm-addon-fit")

        // Set global references
        window.Terminal = xtermModule.Terminal
        window.FitAddon = fitAddonModule.FitAddon

        setIsTerminalReady(true)
      } catch (error) {
        console.error("Failed to load xterm:", error)
      }
    }

    loadXterm()
  }, [])

  // Process command
  const processCommand = (command: string, term: XTerminal): void => {
    const trimmedCommand = command.trim()
    if (!trimmedCommand) return

    const parts = trimmedCommand.split(" ")
    const cmd = parts[0].toLowerCase()
    const args = parts.slice(1)

    if (commands[cmd]) {
      commands[cmd](args, term)
    } else {
      term.writeln(`Command not found: ${cmd}`)
      term.writeln('Type "help" for available commands')
    }
  }

  // Initialize terminal after scripts are loaded
  useEffect(() => {
    if (!isTerminalReady || !terminalRef.current || !window.Terminal || !window.FitAddon) return

    try {
      const Terminal = window.Terminal
      const FitAddon = window.FitAddon

      // Initialize terminal with proper typing
      const term = new Terminal({
        theme: isDarkMode ? terminalThemes.dark : terminalThemes.light,
        fontFamily: 'Menlo, Monaco, "Courier New", monospace',
        fontSize: 14,
        cursorBlink: true,
      })

      // Add fit addon
      const fitAddon = new FitAddon()
      fitAddonRef.current = fitAddon
      term.loadAddon(fitAddon)

      // Open terminal
      term.open(terminalRef.current)

      // Fit terminal after a short delay to ensure DOM is ready
      setTimeout(() => {
        try {
          if (fitAddon && typeof fitAddon.fit === "function") {
            fitAddon.fit()
          }
        } catch (e) {
          console.error("Error fitting terminal:", e)
        }
      }, 100)

      // Write welcome message
      term.writeln("Apache Configuration Terminal")
      term.writeln('Type "help" for available commands')
      term.write("\r\n$ ")

      // Handle input with proper typing
      term.onData((data: string) => {
        if (data === "\r") {
          // Enter key
          term.write("\r\n")

          // Process the command
          processCommand(commandBufferRef.current, term)

          // Reset command buffer
          commandBufferRef.current = ""

          // Show prompt again
          term.write("$ ")
        } else if (data === "\u007f") {
          // Backspace
          if (term.buffer.active.cursorX > 2 && commandBufferRef.current.length > 0) {
            term.write("\b \b")
            commandBufferRef.current = commandBufferRef.current.slice(0, -1)
          }
        } else if (data >= " " && data <= "~") {
          // Printable characters
          term.write(data)
          commandBufferRef.current += data
        }
      })

      setTerminal(term)

      // Resize handler
      const handleResize = (): void => {
        try {
          if (fitAddonRef.current && typeof fitAddonRef.current.fit === "function") {
            fitAddonRef.current.fit()
          }
        } catch (e) {
          console.error("Error during resize:", e)
        }
      }

      window.addEventListener("resize", handleResize)

      // Return cleanup function
      return () => {
        window.removeEventListener("resize", handleResize)
        if (term) {
          try {
            term.dispose()
          } catch (e) {
            console.error("Error disposing terminal:", e)
          }
        }
      }
    } catch (error) {
      console.error("Error initializing terminal:", error)
      return undefined
    }
  }, [isTerminalReady, isDarkMode])

  // Update theme when mode changes
  useEffect(() => {
    if (terminal && isTerminalReady) {
      try {
        terminal.options.theme = isDarkMode ? terminalThemes.dark : terminalThemes.light
      } catch (e) {
        console.error("Error updating terminal theme:", e)
      }
    }
  }, [isDarkMode, terminal, isTerminalReady])

  return (
    <>
      <div ref={terminalRef} className="h-full w-full" />
      {!isTerminalReady && (
        <div className="flex items-center justify-center h-full">
          <div>Loading terminal...</div>
        </div>
      )}
    </>
  )
}

