"use client"

import { Dispatch, SetStateAction, useState } from "react"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Database, Server, Globe, FileCode, Cpu, MoreHorizontal } from "lucide-react"
import { ScrollArea } from "@/components/ui/scroll-area"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Shuriken } from "@/lib/types"

export default function Dashboard({shurikens, setShurikens, index, setIndex, gridView }: { shurikens: Shuriken[], setShurikens: Dispatch<SetStateAction<Shuriken[]>>, index: number, setIndex: Dispatch<SetStateAction<number>>, gridView: "grid" | "list" }) {

  const toggleService = (service_name: string) => {
    setShurikens(
      shurikens.map((service) => {
        if (service.service_name === service_name && service.type.kind === "Daemon") {
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
        <h2 className="text-xl font-semibold mb-3 px-1">Shurikens</h2>
        {gridView === "grid" ? (
        <div className="grid grid-cols-2 sm:grid-cols-2 gap-3 md:gap-4">
          {shurikens.map((service) => (
            <Card key={service.service_name} className="bg-card border-border py-0">
              <CardHeader className="p-3 md:p-4 pb-0 md:pb-2 flex-row items-center justify-between space-y-0">
                <div className="flex items-center gap-1">
                  <div className={`p-1.5 rounded-md bg-muted ${service.status === "running" ? "bg-primary/10" : ""}`}>
                    <service.icon
                      className={`h-4 w-4 ${service.status === "running" ? service.color : "text-muted-foreground"}`}
                    />
                  </div>
                  <CardTitle className="text-sm md:text-base">{service.name}</CardTitle>
                </div>
                {service.type.kind == "Daemon" && <Badge
                  variant={service.status === "running" ? "default" : "secondary"}
                  className={`text-xs ${service.status === "running" ? "bg-primary" : ""}`}
                >
                  {service.status === "running" ? "Running" : "Stopped"}
                </Badge>}
              </CardHeader>
              <CardFooter className={`h-full p-3 pr-2 md:p-4 ${service.type.kind == "Daemon"? "pt-0": "mt-4"} flex gap-2`}>
              <Button
                  variant={service.type.kind == "Daemon" ? (service.status === "running" ? "destructive" : "default") : "outline"}
                  className="text-xs md:text-sm h-8 px-0"
                  style={{ width: "90%"}}
                  onClick={() => toggleService(service.service_name)}
                >
                  {service.type.kind == "Daemon" ? (service.status === "running" ? "Stop" : "Start") : "Manage"}
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
        ) : (
          <Table className="border-border border-2 my-4 rounded-md">
            <TableHeader>
              <TableRow>
                <TableHead className="text-center">Shuriken</TableHead>
                <TableHead className="text-center">Status</TableHead>
                <TableHead className="text-center">Maintenance</TableHead>
                <TableHead className="text-center">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {shurikens.map((service) => (
                <TableRow key={service.service_name} >
                  <TableCell className="text-center">{service.name}</TableCell>
                  <TableCell className="text-center">{service.status}</TableCell>
                  <TableCell className="text-center">{service.maintenance.kind}</TableCell>
                  <TableCell className="flex justify-center"><Button
                  variant={service.type.kind == "Daemon" ? (service.status === "running" ? "destructive" : "default") : "outline"}
                  style={{ width: "40%"}}
                  onClick={() => toggleService(service.service_name)}
                >
                  {service.type.kind == "Daemon" ? (service.status === "running" ? "Stop" : "Start") : "Manage"}
                </Button>
                </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}

      {/* Quick Actions */}
      <Card className="bg-card border-border py-0 my-4">
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
    </div>
  )
}
