"use client"

const TESTING_FLAG = true;

import { ApplicationMenubar } from "@/components/ui/application-menubar"
import { Card, CardContent } from "@/components/ui/card"
import { useState, useRef, useEffect, useCallback } from "react"
import { check } from "@tauri-apps/plugin-updater"
import Dashboard from "@/components/pages/dashboard"
import DeveloperModePanel from "@/components/pages/developer"
import Configuration from "@/components/pages/config"
import Logs from "@/components/pages/logs"
import { HomeIcon, Cog, FileText, Sparkle, Code, Database, Hammer } from "lucide-react"
import UpdateDialog from "@/components/ui/update-dialog"
import Armory from "@/components/pages/armory"
import { Toaster } from "@/components/ui/sonner"
import { useShuriken } from "@/hooks/use-shuriken"
import { useKonami } from "react-konami-code"
import Backup from "@/components/pages/backup"
import Tools from "@/components/pages/tools"
import { UpdateInfo } from "@/lib/types";
// `Button` and `ReactMarkdown` moved into `UpdateDialog`

const mockUpdate = {
  version: "1.12.0",
  date: "2024-10-01",
  downloadAndInstall: () => {
    console.log("Downloading and installing update...")
  },
  body: "• Improved Shuriken startup speed\n• Fixed backup race condition\n• Armory stability improvements",
}

export default function Page() {
  const [devMode, setDevMode] = useState<boolean>(false);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null)
  const [activeIndex, setActiveIndex] = useState(0)
  const [hoverStyle, setHoverStyle] = useState({})
  const [activeStyle, setActiveStyle] = useState({ left: "0px", width: "0px" })
  const tabRefs = useRef<(HTMLDivElement | null)[]>([])
  const [gridView, setGridView] = useState<"grid" | "list">("grid")
  const [platform, setPlatform] = useState<"mac" | "windows" | "linux" | "unknown">("unknown")
  const [showUpdateDialog, setShowUpdateDialog] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>(null)

  const { allShurikens, refreshShurikens, startShuriken, stopShuriken } = useShuriken()

  // Base tabs
  const baseTabs = ["Dashboard", "Configuration", "Logs", "Tools", "Backups", "Armory"];
  const tabs = devMode ? [...baseTabs, "Developer"] : baseTabs;

  useKonami(() => setDevMode(!devMode))

  // Tab icons (add a Code icon for Developer tab)
  const baseTabIcons = [
    <HomeIcon key="home" className={`w-4 h-4 mr-1 ${activeIndex !== 0 ? "dark:text-[#ffffff99]" : "text-red-500"}`} />, 
    <Cog key="cog" className={`w-4 h-4 mr-1 ${activeIndex !== 1 ? "dark:text-[#ffffff99]" : "text-orange-500"}`}/>, 
    <FileText key="file" className={`w-4 h-4 mr-1 ${activeIndex !== 2 ? "dark:text-[#ffffff99]" : "text-green-500"}`}/>,
    <div className="relative">
      <Hammer key="wrench" className={`w-4 h-4 mr-1 -rotate-90 ${activeIndex !== 3 ? "dark:text-[#ffffff99]" : "text-indigo-500"}`}/>
    </div>,
    <Database key="database" className={`w-4 h-4 mr-1 ${activeIndex !== 4 ? "dark:text-[#ffffff99]" : "text-purple-500"}`}/>,
    <div key="zap" className="relative">
      <svg className="w-0 h-0 absolute">
        <defs>
          <linearGradient id="zapStrokeGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f97316" />
            <stop offset="100%" stopColor="#a855f7" />
          </linearGradient>
        </defs>
      </svg>
      <Sparkle className={`mr-1 h-4 w-4 ${activeIndex !== 5 ? "dark:text-[#ffffff99]" : ""}`} style={activeIndex === 5 ? { 
             fill: 'none', 
             stroke: 'url(#zapStrokeGradient)', 
             strokeWidth: 2 
           } : {}}/>
    </div>
  ];

  const tabsIcons = devMode 
    ? [...baseTabIcons, <Code key="dev" className={`w-4 h-4 mr-1 ${activeIndex !== tabs.length - 1 ? "dark:text-[#ffffff99]" : "text-blue-500"}`} />]
    : baseTabIcons;


  const checkForUpdates = async () => {
    try {
      // 🧪 TEST MODE
      if (TESTING_FLAG) {
        setUpdateInfo(mockUpdate)
        setShowUpdateDialog(true)
        return
      }

      // 🚀 REAL UPDATER
      const update = await check()

      if (update) {
        setUpdateInfo({
          version: update.version,
          date: update.date,
          downloadAndInstall: update.downloadAndInstall,
          body: update.body,
        })
        setShowUpdateDialog(true)
      }
    } catch (err) {
      console.error("Update check failed", err)
    }
  }

  // Platform detection
  useEffect(() => {
    const userAgent = window.navigator.userAgent.toLowerCase()
    if (userAgent.includes("mac")) setPlatform("mac")
    else if (userAgent.includes("win")) setPlatform("windows")
    else if (userAgent.includes("linux")) setPlatform("linux")

    requestAnimationFrame(() => {
      const overviewElement = tabRefs.current[0]
      if (overviewElement) {
        const { offsetLeft, offsetWidth } = overviewElement
        setActiveStyle({ left: `${offsetLeft}px`, width: `${offsetWidth}px` })
      }
    })

     checkForUpdates();
  }, [])

  // Hover effect
  useEffect(() => {
    if (hoveredIndex !== null) {
      const hoveredElement = tabRefs.current[hoveredIndex]
      if (hoveredElement) {
        const { offsetLeft, offsetWidth } = hoveredElement
        setHoverStyle({ left: `${offsetLeft}px`, width: `${offsetWidth}px` })
      }
    }
  }, [hoveredIndex])


  // Keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.ctrlKey && !e.shiftKey && e.key === "Tab") {
      e.preventDefault()
      setActiveIndex(prev => (prev + 1) % tabs.length)
    }
    if (e.ctrlKey && e.shiftKey && e.key === "Tab") {
      e.preventDefault()
      setActiveIndex(prev => prev === 0 ? tabs.length - 1 : prev - 1)
    }
  }, [tabs.length])

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Active tab indicator
  useEffect(() => {
    const activeElement = tabRefs.current[activeIndex]
    if (activeElement) {
      const { offsetLeft, offsetWidth } = activeElement
      setActiveStyle({ left: `${offsetLeft}px`, width: `${offsetWidth}px` })
    }
  }, [activeIndex, tabs.length])

  return (
    <div className="relative w-screen h-screen overflow-hidden select-none">
      <ApplicationMenubar 
        platform={platform} 
        gridView={gridView} 
        setGridView={setGridView} 
        activeTab={tabs[activeIndex]} 
        activeWindow="Ninja" 
      />

      <main
        className="absolute w-full overflow-hidden"
        style={{
          top: platform === "mac" ? "28px" : "32px",
          bottom: "0",
          borderBottomLeftRadius: "7px",
          borderBottomRightRadius: "7px",
        }}
      >
        <div className={`flex flex-row select-none items-center ${platform === "mac" ? "pt-2" : "pt-4"}`}>
          <Card className="w-full border-none shadow-none relative flex items-center py-2 justify-center bg-transparent">
            <CardContent className="p-0">
              <div className="relative">
                <div
                  className="absolute h-7.5 transition-all duration-300 ease-out bg-[#0e0f1114] dark:bg-[#ffffff1a] rounded-[6px] flex items-center"
                  style={{ ...hoverStyle, opacity: hoveredIndex !== null ? 1 : 0 }}
                />
                <div
                  className="absolute -bottom-1.5 h-0.5 bg-[#0e0f11] dark:bg-white transition-all duration-300 ease-out"
                  style={activeStyle}
                />
                <div className="relative flex space-x-1.5 items-center">
                  {tabs.map((tab, index) => (
                    <div
                      key={index}
                      ref={(el) => { tabRefs.current[index] = el; return void 0 }}
                      className={`px-3 py-2 cursor-pointer transition-colors duration-300 h-7.5 ${
                        index === activeIndex ? "text-[#0e0e10] dark:text-white" : "text-[#0e0f1199] dark:text-[#ffffff99]"
                      }`}
                      onMouseEnter={() => setHoveredIndex(index)}
                      onMouseLeave={() => setHoveredIndex(null)}
                      onClick={() => setActiveIndex(index)}
                    >
                      <div className="text-sm font-(--www-mattmannucci-me-geist-regular-font-family) leading-5 whitespace-nowrap flex items-center justify-center h-full">
                        {tabsIcons[index]}
                        {tab}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        <Toaster richColors />
        <div className="p-4 overflow-auto" style={{ height: "calc(100vh - 100px)" }}>
          
          <UpdateDialog
            open={showUpdateDialog}
            onOpenChange={setShowUpdateDialog}
            updateInfo={updateInfo}
          />

          {activeIndex === 0 ? (
            <Dashboard gridView={gridView} />
          ) : activeIndex === 1 ? (
            <Configuration/>
          ) : activeIndex === 2 ? (
            <Logs shurikens={allShurikens}/>
          ) : activeIndex === 3 ? (
            <Tools />
          ) : activeIndex === 4 ? (
            <Backup />
          ) : activeIndex === 5 ? (
            <Armory platform={platform} />
          ) : activeIndex === 6 && devMode ? (
            <DeveloperModePanel />
          ) : null}
        </div>
      </main>
    </div>
  )
}
