"use client"

import * as React from "react"
import { cn } from "@/lib/utils"

const TabsContext = React.createContext<{
  selectedIndex: number
  setSelectedIndex: (index: number) => void
  hoveredIndex: number | null
  setHoveredIndex: (index: number | null) => void
  tabRefs: React.MutableRefObject<(HTMLDivElement | null)[]>
  hoverStyle: React.CSSProperties
  activeStyle: React.CSSProperties
}>({
  selectedIndex: 0,
  setSelectedIndex: () => {},
  hoveredIndex: null,
  setHoveredIndex: () => {},
  tabRefs: { current: [] },
  hoverStyle: {},
  activeStyle: {},
})

interface AnimatedTabsProps extends React.HTMLAttributes<HTMLDivElement> {
  defaultValue?: number
  value?: number
  onValueChange?: (value: number) => void
}

const AnimatedTabs = React.forwardRef<HTMLDivElement, AnimatedTabsProps>(
  ({ className, defaultValue = 0, value, onValueChange, ...props }, ref) => {
    const [selectedIndex, setSelectedIndex] = React.useState(defaultValue)
    const [hoveredIndex, setHoveredIndex] = React.useState<number | null>(null)
    const [hoverStyle, setHoverStyle] = React.useState<React.CSSProperties>({})
    const [activeStyle, setActiveStyle] = React.useState<React.CSSProperties>({ left: "0px", width: "0px" })
    const tabRefs = React.useRef<(HTMLDivElement | null)[]>([])

    const handleValueChange = React.useCallback(
      (index: number) => {
        if (value === undefined) {
          setSelectedIndex(index)
        }
        onValueChange?.(index)
      },
      [onValueChange, value],
    )

    React.useEffect(() => {
      if (value !== undefined) {
        setSelectedIndex(value)
      }
    }, [value])

    React.useEffect(() => {
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

    React.useEffect(() => {
      const activeElement = tabRefs.current[selectedIndex]
      if (activeElement) {
        const { offsetLeft, offsetWidth } = activeElement
        setActiveStyle({
          left: `${offsetLeft}px`,
          width: `${offsetWidth}px`,
        })
      }
    }, [selectedIndex])

    React.useEffect(() => {
      requestAnimationFrame(() => {
        const firstElement = tabRefs.current[0]
        if (firstElement) {
          const { offsetLeft, offsetWidth } = firstElement
          setActiveStyle({
            left: `${offsetLeft}px`,
            width: `${offsetWidth}px`,
          })
        }
      })
    }, [])

    return (
      <TabsContext.Provider
        value={{
          selectedIndex,
          setSelectedIndex: handleValueChange,
          hoveredIndex,
          setHoveredIndex,
          tabRefs,
          hoverStyle,
          activeStyle,
        }}
      >
        <div ref={ref} className={cn("relative", className)} {...props} />
      </TabsContext.Provider>
    )
  },
)
AnimatedTabs.displayName = "AnimatedTabs"

type AnimatedTabsListProps = React.HTMLAttributes<HTMLDivElement>

const AnimatedTabsList = React.forwardRef<HTMLDivElement, AnimatedTabsListProps>(({ className, ...props }, ref) => {
  const { hoverStyle, hoveredIndex, activeStyle } = React.useContext(TabsContext)

  return (
    <div ref={ref} className={cn("relative flex space-x-[6px] items-center", className)} {...props}>
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
    </div>
  )
})
AnimatedTabsList.displayName = "AnimatedTabsList"

interface AnimatedTabsTriggerProps extends React.HTMLAttributes<HTMLDivElement> {
  index: number
}

const AnimatedTabsTrigger = React.forwardRef<HTMLDivElement, AnimatedTabsTriggerProps>(
  ({ className, index, ...props }, ref) => {
    const { selectedIndex, setSelectedIndex, setHoveredIndex, tabRefs } = React.useContext(TabsContext)
    const isActive = selectedIndex === index

    return (
      <div
        ref={(el) => {
          if (typeof ref === "function") {
            ref(el)
          } else if (ref) {
            ref.current = el
          }
          tabRefs.current[index] = el
        }}
        className={cn(
          "px-3 py-2 cursor-pointer transition-colors duration-300 h-[30px]",
          isActive ? "text-[#0e0e10] dark:text-white" : "text-[#0e0f1199] dark:text-[#ffffff99]",
          className,
        )}
        onMouseEnter={() => setHoveredIndex(index)}
        onMouseLeave={() => setHoveredIndex(null)}
        onClick={() => setSelectedIndex(index)}
        {...props}
      />
    )
  },
)
AnimatedTabsTrigger.displayName = "AnimatedTabsTrigger"

interface AnimatedTabsContentProps extends React.HTMLAttributes<HTMLDivElement> {
  index: number
}

const AnimatedTabsContent = React.forwardRef<HTMLDivElement, AnimatedTabsContentProps>(
  ({ className, index, children, ...props }, ref) => {
    const { selectedIndex } = React.useContext(TabsContext)
    const isActive = selectedIndex === index

    if (!isActive) return null

    return (
      <div ref={ref} className={cn("mt-2 animate-in fade-in-50 duration-300", className)} {...props}>
        {children}
      </div>
    )
  },
)
AnimatedTabsContent.displayName = "AnimatedTabsContent"

export { AnimatedTabs, AnimatedTabsList, AnimatedTabsTrigger, AnimatedTabsContent }
