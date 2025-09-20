"use client"

import * as React from "react"
import { Window } from "@tauri-apps/api/window"
import { Anvil, File, FileText, FolderOpen, Github, HelpCircle, Laptop, LayoutGrid, LifeBuoy, LogOut, Moon, Save, Sparkle, Sun } from 'lucide-react'
import { invoke } from "@tauri-apps/api/core"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useTheme } from "next-themes"
import { WindowControls } from "./window-controls"
import { Dispatch, SetStateAction } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import Image from "next/image"

export function ApplicationMenubar({ platform, gridView, setGridView, activeTab, activeWindow }: { platform: "mac" | "windows" | "linux" | "unknown", gridView?: "grid" | "list", setGridView?: Dispatch<SetStateAction<"grid" | "list">>, activeTab?: string, activeWindow: string }) {
  const { setTheme, theme } = useTheme()

  return (
    <div 
      className={`fixed flex z-[999] ${platform === "mac" ? "h-8" : "h-10"} justify-between items-center border-b bg-background drag w-full`} 
      style={{ 
        borderTopLeftRadius: '7px', 
        borderTopRightRadius: '7px' 
      }}
      data-tauri-drag-region
    >
      {platform === "mac" && (
        <WindowControls
          onMinimize={() => Window.getCurrent().minimize()}
          onClose={() => Window.getCurrent().close()}
        />
      )}

      {platform !== "mac" && (
      <div className="flex items-center ml-0 gap-2 px-4">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-8 px-2 text-sm">
              File
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-56">
            <DropdownMenuItem>
              <FileText className="mr-2 h-4 w-4" />
              <span>New</span>
              <DropdownMenuShortcut>⌘N</DropdownMenuShortcut>
            </DropdownMenuItem>
            <DropdownMenuItem>
              <FolderOpen className="mr-2 h-4 w-4" />
              <span>Open</span>
              <DropdownMenuShortcut>⌘O</DropdownMenuShortcut>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <span>Export</span>
              <DropdownMenuShortcut>⌘E</DropdownMenuShortcut>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => activeWindow == "Forge" || activeWindow == "Armory"? invoke("toggle_"+activeWindow.toLowerCase()+"_window").catch((reason) => console.error(reason)) :Window.getCurrent().close()}>
              <LogOut className="mr-2 h-4 w-4" />
              <span>Exit</span>
              <DropdownMenuShortcut>⌘Q</DropdownMenuShortcut>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-8 px-2 text-sm">
              View
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-56">
            {activeWindow == "Ninja" && (
              <>
              <DropdownMenuRadioGroup value={gridView} onValueChange={setGridView as (value: string) => void}>
              <DropdownMenuRadioItem value="grid">
                <LayoutGrid className="mr-2 h-4 w-4" />
                <span>Grid View</span>
              </DropdownMenuRadioItem>
              <DropdownMenuRadioItem value="list">
                <File className="mr-2 h-4 w-4" />
                <span>List View</span>
              </DropdownMenuRadioItem>
            </DropdownMenuRadioGroup>
            <DropdownMenuSeparator />
            </>
            )}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger>
                <span>Theme</span>
              </DropdownMenuSubTrigger>
              <DropdownMenuPortal>
                <DropdownMenuSubContent>
                  <DropdownMenuItem onClick={() => setTheme("light")}>
                    <Sun className="mr-2 h-4 w-4" />
                    <span>Light</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setTheme("dark")}>
                    <Moon className="mr-2 h-4 w-4" />
                    <span>Dark</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setTheme("system")}>
                    <Laptop className="mr-2 h-4 w-4" />
                    <span>System</span>
                  </DropdownMenuItem>
                </DropdownMenuSubContent>
              </DropdownMenuPortal>
            </DropdownMenuSub>
          </DropdownMenuContent>
        </DropdownMenu>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-8 px-2 text-sm">
              Help
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-56">
            <DropdownMenuItem onClick={() => {
              // For Tauri: open in default browser, not in-app webview
              openUrl("https://ninja-rs.vercel.app/docs")
            }}>
              <HelpCircle className="mr-2 h-4 w-4" />
              <span>Documentation</span>
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={async () => {
              // For Tauri: open in default browser, not in-app webview
                openUrl("https://github.com/tunafysh/ninja/issues")

              }}
            >
              <LifeBuoy className="mr-2 h-4 w-4" />
              <span>Support</span>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => {
              // For Tauri: open in default browser, not in-app webview
              openUrl("https://github.com/tunafysh/ninja");
            }
            }>
                <Github className="mr-2 h-4 w-4" />
                <span>GitHub</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
        )}

        {platform != "mac" && (
          <p className="text-sm text-muted-foreground mr-32 select-none">
            {activeTab != undefined? activeTab+" - "+activeWindow: activeWindow}
          </p>

        )}

        <div>
        {platform !== "mac" && (
          <div className="flex items-center">
            <WindowControls
              //@ts-ignore
              onMinimize={() => Window.getCurrent().minimize()}
              //@ts-ignore
              onClose={() => activeWindow == "Forge" || activeWindow == "Armory"? invoke("toggle_"+activeWindow.toLowerCase()+"_window").catch((reason) => console.error(reason)) :Window.getCurrent().close()}
            />
          </div>
        )}
        </div>
    </div>
  )
}
