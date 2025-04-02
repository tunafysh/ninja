"use client"

import React, { useState, useCallback, useRef } from "react"
import { DndProvider, useDrag, useDrop } from "react-dnd"
import { HTML5Backend } from "react-dnd-html5-backend"
import {
  BarChart3,
  Calendar,
  FileText,
  Grip,
  Home,
  Image,
  LayoutDashboard,
  Mail,
  MessageSquare,
  Settings,
  Users,
  Plus,
  Trash,
  Edit,
  Code,
  Terminal,
  Bookmark,
  Briefcase,
  Cloud,
  CreditCard,
  Database,
  Download,
  FileCode,
  Film,
  Folder,
  Globe,
  HardDrive,
  Headphones,
  Heart,
  Link,
  Map,
  Music,
  Package,
  Phone,
  PieChart,
  Printer,
  Search,
  Send,
  Server,
  ShoppingCart,
  Smartphone,
  Star,
  Tag,
  Upload,
  Video,
  Zap,
} from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { ScrollArea } from "@/components/ui/scroll-area"

// Define the type for our tool items
type ToolItem = {
  id: string
  name: string
  icon: React.ReactNode
  color?: string
  command?: string
}

// Map of available icons
const iconMap: Record<string, React.ReactNode> = {
  Home: <Home className="h-5 w-5" />,
  Dashboard: <LayoutDashboard className="h-5 w-5" />,
  Messages: <MessageSquare className="h-5 w-5" />,
  Mail: <Mail className="h-5 w-5" />,
  Calendar: <Calendar className="h-5 w-5" />,
  Analytics: <BarChart3 className="h-5 w-5" />,
  Files: <FileText className="h-5 w-5" />,
  Images: <Image className="h-5 w-5" />,
  Users: <Users className="h-5 w-5" />,
  Settings: <Settings className="h-5 w-5" />,
  Plus: <Plus className="h-5 w-5" />,
  Trash: <Trash className="h-5 w-5" />,
  Edit: <Edit className="h-5 w-5" />,
  Code: <Code className="h-5 w-5" />,
  Terminal: <Terminal className="h-5 w-5" />,
  Bookmark: <Bookmark className="h-5 w-5" />,
  Briefcase: <Briefcase className="h-5 w-5" />,
  Cloud: <Cloud className="h-5 w-5" />,
  CreditCard: <CreditCard className="h-5 w-5" />,
  Database: <Database className="h-5 w-5" />,
  Download: <Download className="h-5 w-5" />,
  FileCode: <FileCode className="h-5 w-5" />,
  Film: <Film className="h-5 w-5" />,
  Folder: <Folder className="h-5 w-5" />,
  Globe: <Globe className="h-5 w-5" />,
  HardDrive: <HardDrive className="h-5 w-5" />,
  Headphones: <Headphones className="h-5 w-5" />,
  Heart: <Heart className="h-5 w-5" />,
  Link: <Link className="h-5 w-5" />,
  Map: <Map className="h-5 w-5" />,
  Music: <Music className="h-5 w-5" />,
  Package: <Package className="h-5 w-5" />,
  Phone: <Phone className="h-5 w-5" />,
  PieChart: <PieChart className="h-5 w-5" />,
  Printer: <Printer className="h-5 w-5" />,
  Search: <Search className="h-5 w-5" />,
  Send: <Send className="h-5 w-5" />,
  Server: <Server className="h-5 w-5" />,
  ShoppingCart: <ShoppingCart className="h-5 w-5" />,
  Smartphone: <Smartphone className="h-5 w-5" />,
  Star: <Star className="h-5 w-5" />,
  Tag: <Tag className="h-5 w-5" />,
  Upload: <Upload className="h-5 w-5" />,
  Video: <Video className="h-5 w-5" />,
  Zap: <Zap className="h-5 w-5" />,
}

// Available colors
const colorOptions = [
  { value: "default", label: "Default" },
  { value: "text-blue-500", label: "Blue" },
  { value: "text-green-500", label: "Green" },
  { value: "text-yellow-500", label: "Yellow" },
  { value: "text-purple-500", label: "Purple" },
  { value: "text-pink-500", label: "Pink" },
  { value: "text-orange-500", label: "Orange" },
  { value: "text-cyan-500", label: "Cyan" },
  { value: "text-indigo-500", label: "Indigo" },
  { value: "text-red-500", label: "Red" },
]

// Initial tools data
const initialTools: ToolItem[] = [
  { id: "start", name: "Start Server", icon: <Home className="h-5 w-5" />, command: "sudo systemctl start apache2" },   
  
]

// Drag item type
const ItemTypes = {
  TOOL: "tool",
}

// Draggable tool component
const DraggableTool = ({
  id,
  index,
  tool,
  moveItem,
  onRemove,
  onEdit,
}: {
  id: string
  index: number
  tool: ToolItem
  moveItem: (dragIndex: number, hoverIndex: number) => void
  onRemove: (id: string) => void
  onEdit: (tool: ToolItem) => void
}) => {
  const ref = React.useRef<HTMLDivElement>(null)

  const [{ isDragging }, drag, preview] = useDrag({
    type: ItemTypes.TOOL,
    item: () => ({ id, index }),
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
    }),
  })

  const [, drop] = useDrop({
    accept: ItemTypes.TOOL,
    hover: (item: { id: string; index: number }, monitor) => {
      if (!ref.current) {
        return
      }
      const dragIndex = item.index
      const hoverIndex = index

      // Don't replace items with themselves
      if (dragIndex === hoverIndex) {
        return
      }

      // Determine rectangle on screen
      const hoverBoundingRect = ref.current?.getBoundingClientRect()

      // Get vertical middle
      const hoverMiddleY = (hoverBoundingRect.bottom - hoverBoundingRect.top) / 2

      // Determine mouse position
      const clientOffset = monitor.getClientOffset()

      // Get pixels to the top
      const hoverClientY = clientOffset!.y - hoverBoundingRect.top

      // Only perform the move when the mouse has crossed half of the items height
      // When dragging downwards, only move when the cursor is below 50%
      // When dragging upwards, only move when the cursor is above 50%

      // Dragging downwards
      if (dragIndex < hoverIndex && hoverClientY < hoverMiddleY) {
        return
      }

      // Dragging upwards
      if (dragIndex > hoverIndex && hoverClientY > hoverMiddleY) {
        return
      }

      // Time to actually perform the action
      moveItem(dragIndex, hoverIndex)

      // Note: we're mutating the monitor item here!
      // Generally it's better to avoid mutations,
      // but it's good here for the sake of performance
      // to avoid expensive index searches.
      item.index = hoverIndex
    },
  })

  // Initialize drag and drop refs
  drag(drop(ref))

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <TooltipProvider delayDuration={300}>
          <Tooltip>
            <TooltipTrigger asChild>
              <div
                ref={ref}
                className={cn(
                  "group flex cursor-grab items-center justify-between rounded-md p-2 transition-colors hover:bg-muted",
                  isDragging ? "opacity-50" : "opacity-100",
                )}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "flex h-9 w-9 items-center justify-center rounded-md bg-muted",
                      tool.color === "default" ? "" : tool.color,
                    )}
                  >
                    {tool.icon}
                  </div>
                  <span className="text-sm font-medium">{tool.name}</span>
                </div>
                <Grip className="h-4 w-4 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
              </div>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>Drag to reorder</p>
              <p className="text-xs text-muted-foreground">Right-click for options</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={() => onEdit(tool)}>
          <Edit className="mr-2 h-4 w-4" />
          Edit
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onRemove(id)} className="text-destructive focus:text-destructive">
          <Trash className="mr-2 h-4 w-4" />
          Remove
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}

// Add Tool Dialog Component
function AddToolDialog({
  open,
  onOpenChange,
  onAdd,
  editingTool = null,
  onUpdate,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onAdd: (tool: Omit<ToolItem, "id">) => void
  editingTool?: ToolItem | null
  onUpdate?: (id: string, tool: Omit<ToolItem, "id">) => void
}) {
  const [name, setName] = useState(editingTool?.name || "")
  const [command, setCommand] = useState(editingTool?.command || "")
  const [selectedIcon, setSelectedIcon] = useState(
    editingTool
      ? Object.keys(iconMap).find(
          (key) =>
            React.isValidElement(iconMap[key]) &&
            React.isValidElement(editingTool.icon) &&
            iconMap[key].type === editingTool.icon.type,
        ) || "Home"
      : "Home",
  )
  const [color, setColor] = useState(editingTool?.color || "default")

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    const newTool = {
      name,
      icon: iconMap[selectedIcon],
      command,
      color: color === "default" ? "" : color,
    }

    if (editingTool && onUpdate) {
      onUpdate(editingTool.id, newTool)
    } else {
      onAdd(newTool)
    }

    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{editingTool ? "Edit Tool" : "Add New Tool"}</DialogTitle>
          <DialogDescription>
            {editingTool ? "Update this tool's details." : "Create a new tool for your sidebar."}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="name" className="text-right">
                Name
              </Label>
              <Input id="name" value={name} onChange={(e) => setName(e.target.value)} className="col-span-3" required />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="command" className="text-right">
                Command
              </Label>
              <Input
                id="command"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                className="col-span-3"
                placeholder="/command or https://..."
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="icon" className="text-right">
                Icon
              </Label>
              <Select value={selectedIcon} onValueChange={setSelectedIcon}>
                <SelectTrigger className="col-span-3">
                  <SelectValue placeholder="Select an icon" />
                </SelectTrigger>
                <SelectContent>
                  <ScrollArea className="h-72">
                    <SelectGroup>
                      <SelectLabel>Icons</SelectLabel>
                      {Object.keys(iconMap).map((iconName) => (
                        <SelectItem key={iconName} value={iconName}>
                          <div className="flex items-center">
                            <div className="mr-2">{iconMap[iconName]}</div>
                            <span>{iconName}</span>
                          </div>
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </ScrollArea>
                </SelectContent>
              </Select>
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="color" className="text-right">
                Color
              </Label>
              <Select value={color} onValueChange={setColor}>
                <SelectTrigger className="col-span-3">
                  <SelectValue placeholder="Select a color" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectLabel>Colors</SelectLabel>
                    {colorOptions.map((colorOption) => (
                      <SelectItem key={colorOption.value} value={colorOption.value}>
                        <div className="flex items-center">
                          <div
                            className={cn(
                              "mr-2 h-4 w-4 rounded-full",
                              colorOption.value === "default" ? "bg-muted" : colorOption.value,
                            )}
                          />
                          <span>{colorOption.label}</span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button type="submit">{editingTool ? "Update" : "Add"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function SwappableSidebar() {
  const [tools, setTools] = useState(initialTools)
  const [isCollapsed, setIsCollapsed] = useState(false)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingTool, setEditingTool] = useState<ToolItem | null>(null)
  const sidebarRef = useRef<HTMLDivElement>(null)

  const moveItem = useCallback((dragIndex: number, hoverIndex: number) => {
    setTools((prevTools) => {
      const newTools = [...prevTools]
      const draggedItem = newTools[dragIndex]
      newTools.splice(dragIndex, 1)
      newTools.splice(hoverIndex, 0, draggedItem)
      return newTools
    })
  }, [])

  const resetOrder = () => {
    setTools(initialTools)
  }

  const handleAddTool = (tool: Omit<ToolItem, "id">) => {
    const id = `tool-${Date.now()}`
    setTools((prevTools) => [...prevTools, { id, ...tool }])
  }

  const handleRemoveTool = (id: string) => {
    setTools((prevTools) => prevTools.filter((tool) => tool.id !== id))
  }

  const handleEditTool = (tool: ToolItem) => {
    setEditingTool(tool)
    setDialogOpen(true)
  }

  const handleUpdateTool = (id: string, updatedTool: Omit<ToolItem, "id">) => {
    setTools((prevTools) => prevTools.map((tool) => (tool.id === id ? { ...tool, ...updatedTool } : tool)))
    setEditingTool(null)
  }

  const handleContextMenu = (e: React.MouseEvent) => {
    // Only open context menu if clicking on the sidebar background, not on a tool
    if (e.target === sidebarRef.current || sidebarRef.current?.contains(e.target as Node)) {
      const isToolElement = (e.target as HTMLElement).closest("[data-tool]")
      if (!isToolElement) {
        e.preventDefault()
        setEditingTool(null)
        setDialogOpen(true)
      }
    }
  }

  return (
    <DndProvider backend={HTML5Backend}>
      <div
        ref={sidebarRef}
        className={cn(
          "flex h-screen flex-col border-r bg-background transition-all duration-300",
          isCollapsed ? "w-16" : "w-64",
        )}
        onContextMenu={handleContextMenu}
      >
        <div className="flex h-14 items-center justify-between border-b px-4">
          <h2 className={cn("text-lg font-semibold", isCollapsed && "hidden")}>Quick Tools</h2>
          <div className="flex items-center gap-2">
            {!isCollapsed && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  setEditingTool(null)
                  setDialogOpen(true)
                }}
                className="h-8 w-8 p-0"
                title="Add Tool"
              >
                <Plus className="h-4 w-4" />
              </Button>
            )}
            <Button variant="ghost" size="icon" onClick={() => setIsCollapsed(!isCollapsed)} className="h-8 w-8 p-0">
              {isCollapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-auto p-2">
          {tools.map((tool, index) =>
            isCollapsed ? (
              <TooltipProvider key={tool.id} delayDuration={300}>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="icon" className={cn("mb-1 h-12 w-12", tool.color)}>
                      {tool.icon}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    <p>{tool.name}</p>
                    {tool.command && <p className="text-xs text-muted-foreground">{tool.command}</p>}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            ) : (
              <div key={tool.id} data-tool="true">
                <DraggableTool
                  id={tool.id}
                  index={index}
                  tool={tool}
                  moveItem={moveItem}
                  onRemove={handleRemoveTool}
                  onEdit={handleEditTool}
                />
              </div>
            ),
          )}
        </div>

        {!isCollapsed && (
          <div className="border-t p-2">
            <Button variant="outline" size="sm" onClick={resetOrder} className="w-full">
              Reset Order
            </Button>
          </div>
        )}
      </div>

      <AddToolDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={handleAddTool}
        editingTool={editingTool}
        onUpdate={handleUpdateTool}
      />
    </DndProvider>
  )
}

// Add missing icons
function ChevronLeft(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="m15 18-6-6 6-6" />
    </svg>
  )
}

function ChevronRight(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  )
}

