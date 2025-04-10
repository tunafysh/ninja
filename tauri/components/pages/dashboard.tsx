"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Database, Server, Globe, FileCode, Cpu, MoreHorizontal } from "lucide-react"
import { ScrollArea } from "@/components/ui/scroll-area"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"

export default function Dashboard() {
  const [services, setServices] = useState([
    { id: "apache", name: "Apache", status: "stopped", icon: Server, color: "text-red-500" },
    { id: "mysql", name: "MySQL", status: "stopped", icon: Database, color: "text-blue-500" },
    { id: "php", name: "PHP", status: "stopped", icon: FileCode, color: "text-purple-500" },
    { id: "filezilla", name: "FileZilla", status: "stopped", icon: Globe, color: "text-green-500" },
  ])

  const toggleService = (id: string) => {
    setServices(
      services.map((service) => {
        if (service.id === id) {
          const newStatus = service.status === "running" ? "stopped" : "running"
          return { ...service, status: newStatus }
        }
        return service
      }),
    )
  }

  return (
    <div className="space-y-4 md:space-y-6">
      {/* Services Section */}
      <div>
        <h2 className="text-xl font-semibold mb-3 px-1">Services</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 md:gap-4">
          {services.map((service) => (
            <Card key={service.id} className="bg-card border-border overflow-hidden py-0">
              <CardHeader className="p-3 md:p-4 pb-0 md:pb-2 flex-row items-center justify-between space-y-0">
                <div className="flex items-center gap-1">
                  <div className={`p-1.5 rounded-md bg-muted ${service.status === "running" ? "bg-primary/10" : ""}`}>
                    <service.icon
                      className={`h-4 w-4 ${service.status === "running" ? service.color : "text-muted-foreground"}`}
                    />
                  </div>
                  <CardTitle className="text-sm md:text-base">{service.name}</CardTitle>
                </div>
                <Badge
                  variant={service.status === "running" ? "default" : "secondary"}
                  className={`text-xs ${service.status === "running" ? "bg-primary" : ""}`}
                >
                  {service.status === "running" ? "Running" : "Stopped"}
                </Badge>
              </CardHeader>
              <CardFooter className="p-3 md:p-4 pt-0 flex gap-2">
                <Button
                  variant={service.status === "running" ? "destructive" : "default"}
                  className="text-xs md:text-sm h-8 px-0"
                  style={{ width: "90%"}}
                  onClick={() => toggleService(service.id)}
                >
                  {service.status === "running" ? "Stop" : "Start"}
                </Button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="outline" size="sm" className="px-2 h-8">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem>Restart</DropdownMenuItem>
                    <DropdownMenuItem>View Logs</DropdownMenuItem>
                    <DropdownMenuItem>Configure</DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </CardFooter>
            </Card>
          ))}
        </div>
      </div>

      {/* Quick Actions */}
      <Card className="bg-card border-border py-0">
        <CardHeader className="p-3 md:p-4 pb-0 md:pb-2">
          <CardTitle className="text-base md:text-lg flex items-center gap-2">
            <Cpu className="h-4 w-4 md:h-5 md:w-5 text-primary" />
            Quick Actions
          </CardTitle>
        </CardHeader>
        <CardContent className="p-3 md:p-4 grid grid-cols-1 sm:grid-cols-3 gap-2">
          <Button variant="outline" className="w-full justify-start text-xs md:text-sm h-9">
            <Globe className="mr-2 h-4 w-4 flex-shrink-0" />
            <span className="truncate">Open localhost</span>
          </Button>
          <Button variant="outline" className="w-full justify-start text-xs md:text-sm h-9">
            <Database className="mr-2 h-4 w-4 flex-shrink-0" />
            <span className="truncate">phpMyAdmin</span>
          </Button>
          <Button variant="outline" className="w-full justify-start text-xs md:text-sm h-9">
            <Server className="mr-2 h-4 w-4 flex-shrink-0" />
            <span className="truncate">Restart All</span>
          </Button>
        </CardContent>
      </Card>

      {/* Local Projects */}
      <Card className="bg-card border-border py-0">
        <CardHeader className="p-3 md:p-4 pb-0 md:pb-2">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-base md:text-lg">Local Projects</CardTitle>
              <CardDescription className="text-xs md:text-sm">Your web projects in htdocs directory</CardDescription>
            </div>
            <Button variant="outline" size="sm" className="h-8 text-xs md:text-sm hidden sm:flex">
              View All
            </Button>
          </div>
        </CardHeader>
        <CardContent className="p-3 md:p-4">
          <ScrollArea className="w-full" type="always">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 md:gap-4 min-w-[600px]">
              {["Project 1", "WordPress Site", "Laravel App", "React App", "Vue Project"].map((project, index) => (
                <Card key={index} className="bg-muted/50 border-border py-0">
                  <CardHeader className="p-3 pb-0">
                    <CardTitle className="text-sm md:text-base truncate">{project}</CardTitle>
                  </CardHeader>
                  <CardFooter className="p-3 pt-2 flex justify-between">
                    <Button variant="ghost" size="sm" className="h-7 text-xs px-2">
                      <Globe className="mr-1 h-3 w-3" />
                      Open
                    </Button>
                    <Button variant="ghost" size="sm" className="h-7 text-xs px-2">
                      <FileCode className="mr-1 h-3 w-3" />
                      Files
                    </Button>
                  </CardFooter>
                </Card>
              ))}
            </div>
          </ScrollArea>
          <Button variant="outline" size="sm" className="w-full mt-3 sm:hidden">
            View All Projects
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}
