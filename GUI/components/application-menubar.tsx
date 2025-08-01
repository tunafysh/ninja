"use client"

import * as React from "react"
// import { Window } from "@tauri-apps/api/window"
import { File, FileText, FolderOpen, Github, HelpCircle, Laptop, LayoutGrid, LifeBuoy, LogOut, Moon, Save, Sun } from 'lucide-react'

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

export function ApplicationMenubar({ platform, gridView, setGridView }: { platform: "mac" | "windows" | "linux" | "unknown", gridView: "grid" | "list", setGridView: Dispatch<SetStateAction<"grid" | "list">> }) {
  const { setTheme, theme } = useTheme()
  const [viewMode, setViewMode] = React.useState("grid")

  return (
    <div 
      className={`fixed flex z-50 ${platform === "mac" ? "h-8" : "h-10"} justify-between items-center border-b bg-background drag w-full`} 
      style={{ 
        borderTopLeftRadius: '7px', 
        borderTopRightRadius: '7px' 
      }}
      data-tauri-drag-region
    >
      {/* {platform === "mac" && (
        <WindowControls
          onMinimize={() => Window.getCurrent().minimize()}
          onMaximize={() => Window.getCurrent().maximize()}
          onClose={() => Window.getCurrent().close()}
        />
      )} */}

      {platform !== "mac" && (
      <div className="flex items-center gap-2 px-4">
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
            <DropdownMenuItem>
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
            <DropdownMenuItem onClick={() => window.open("https://ninja-rs.vercel.app/docs", "_blank")}>
              <HelpCircle className="mr-2 h-4 w-4" />
              <span>Documentation</span>
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => {
              // For Tauri: open in default browser, not in-app webview
                window.open("https://ninja-rs.vercel.app/support", "_blank");

              }}
            >
              <LifeBuoy className="mr-2 h-4 w-4" />
              <span>Support</span>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <Github className="mr-2 h-4 w-4" />
              <span>GitHub</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
        )}
        
        <div className="ml-auto">
        {platform !== "mac" && (
          <div className="flex items-center">
            <WindowControls
              onMinimize={() => console.log("Minimize")}
              onMaximize={() => console.log("Maximize")}
              onClose={() => console.log("Close")}
            />
          </div>
        )}
        </div>
    </div>
  )
}
