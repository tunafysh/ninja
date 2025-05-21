"use client"
// import { warn, debug, trace, info, error } from '@tauri-apps/plugin-log';
import { ApplicationMenubar } from "@/components/application-menubar"
import { Card, CardContent } from "@/components/ui/card"
import { useState, useRef, useEffect } from "react"
import { ModeToggle } from "@/components/ui/themetoggle"
import Dashboard from "@/components/pages/dashboard"
import Configuration from "@/components/pages/config"
import Tools from "@/components/pages/tools"
import Logs from "@/components/pages/logs"
import { HomeIcon, Cog, FileText, Wrench, Zap } from "lucide-react"
import Scripting from "@/components/pages/scripting"

const tabIcons = [ <HomeIcon className="w-4 h-4 mr-1" />, <Cog className="w-4 h-4 mr-1"/>, <FileText className="w-4 h-4 mr-1"/>, <Wrench className="w-4 h-4 mr-1"/>, <Zap className="w-4 h-4 mr-1"/> ]
const tabs = ["Dashboard", "Configuration", "Logs", "Tools", "Scripting"]
export default function Page() {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null)
  const [activeIndex, setActiveIndex] = useState(0)
  const [hoverStyle, setHoverStyle] = useState({})
  const [activeStyle, setActiveStyle] = useState({ left: "0px", width: "0px" })
  const tabRefs = useRef<(HTMLDivElement | null)[]>([])
  const [platform, setPlatform] = useState<"mac" | "windows" | "linux" | "unknown">("unknown")

  function forwardConsole(
    fnName: 'log' | 'debug' | 'info' | 'warn' | 'error',
    logger: (message: string) => Promise<void>
  ) {
    const original = console[fnName];
    console[fnName] = (message) => {
      original(message);
      logger(message);
    };
  }

  useEffect(() => {
    // Detect platform
    const userAgent = window.navigator.userAgent.toLowerCase()
    if (userAgent.indexOf("mac") !== -1) {
      setPlatform("mac")
    } else if (userAgent.indexOf("win") !== -1) {
      setPlatform("windows")
    } else if (userAgent.indexOf("linux") !== -1) {
      setPlatform("linux")
    }

    // forwardConsole('log', trace);
    // forwardConsole('debug', debug);
    // forwardConsole('info', info);
    // forwardConsole('warn', warn);
    // forwardConsole('error', error);
  })

  useEffect(() => {
    if (hoveredIndex !== null) {
      const hoveredElement = tabRefs.current[hoveredIndex]
      if (hoveredElement) {
        const { offsetLeft, offsetWidth } = hoveredElement
        setHoverStyle({
          left: `${offsetLeft}px`,
          width: `${offsetWidth}px`,
        })
      }
    }
  }, [hoveredIndex])

  useEffect(() => {
    const activeElement = tabRefs.current[activeIndex]
    if (activeElement) {
      const { offsetLeft, offsetWidth } = activeElement
      setActiveStyle({
        left: `${offsetLeft}px`,
        width: `${offsetWidth}px`,
      })
    }
  }, [activeIndex])

  useEffect(() => {
    requestAnimationFrame(() => {
      const overviewElement = tabRefs.current[0]
      if (overviewElement) {
        const { offsetLeft, offsetWidth } = overviewElement
        setActiveStyle({
          left: `${offsetLeft}px`,
          width: `${offsetWidth}px`,
        })
      }
    })
  }, [])

  return (
    <div className="relative w-screen h-screen overflow-hidden">
      {/* Application Menubar at the top */}
      <ApplicationMenubar platform={platform} />

      {/* Main content container with rounded bottom corners */}
      <main
        className="absolute w-full overflow-hidden"
        style={{
          top: platform === "mac" ? "28px" : "32px",
          bottom: "0",
          borderBottomLeftRadius: "7px",
          borderBottomRightRadius: "7px",
        }}
      >
        <div className={`flex flex-row items-center ${platform === "mac" ? "pt-2" : "pt-4"}`}>
          <Card className="w-full border-none shadow-none relative flex items-center py-2 justify-center bg-transparent">
            <CardContent className="p-0">
              <div className="relative">
                {/* Hover Highlight */}
                <div
                  className="absolute h-[30px] transition-all duration-300 ease-out bg-[#0e0f1114] dark:bg-[#ffffff1a] rounded-[6px] flex items-center"
                  style={{
                    ...hoverStyle,
                    opacity: hoveredIndex !== null ? 1 : 0,
                  }}
                />

                {/* Active Indicator */}
                <div
                  className="absolute bottom-[-6px] h-[2px] bg-[#0e0f11] dark:bg-white transition-all duration-300 ease-out"
                  style={activeStyle}
                />

                {/* Tabs */}
                <div className="relative flex space-x-[6px] items-center">
                  {tabs.map((tab, index) => (
                    <div
                      key={index}
                      ref={(el) => {
                        tabRefs.current[index] = el
                        return void 0
                      }}
                      className={`px-3 py-2 cursor-pointer transition-colors duration-300 h-[30px] ${
                        index === activeIndex
                          ? "text-[#0e0e10] dark:text-white"
                          : "text-[#0e0f1199] dark:text-[#ffffff99]"
                      }`}
                      onMouseEnter={() => setHoveredIndex(index)}
                      onMouseLeave={() => setHoveredIndex(null)}
                      onClick={() => setActiveIndex(index)}
                    >
                      <div className="text-sm font-[var(--www-mattmannucci-me-geist-regular-font-family)] leading-5 whitespace-nowrap flex items-center justify-center h-full">
                        {tabIcons[index]}
                        {tab}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
          <ModeToggle />
        </div>

        {/* Content area with scrolling */}
        <div className="p-4 overflow-auto scrollbar-hidden
        
        
        
        
        
        
        " style={{ height: "calc(100vh - 100px)" }}>
          {activeIndex === 0 ? (
            <Dashboard />
          ) : activeIndex === 1 ? (
            <Configuration />
          ) : activeIndex === 2 ? (
            <Logs />
          ) : activeIndex === 3 ? (
            <Tools />
          ) : activeIndex === 4 ? (
            <Scripting />
          ) : null}
        </div>
      </main>
    </div>
  )
}
