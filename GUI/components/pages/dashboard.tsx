"use client"

import { Dispatch, SetStateAction, useEffect } from "react"

import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Globe, FileCode, MoreHorizontal, LucideIcon, RefreshCcw } from "lucide-react"
import { DynamicIcon } from "lucide-react/dynamic"
import { ScrollArea } from "@/components/ui/scroll-area"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Shuriken } from "@/lib/types"
import { useShuriken } from "@/hooks/use-shuriken"

export default function Dashboard({shurikens, gridView, onRefresh }: { shurikens: Shuriken[], gridView: "grid" | "list", onRefresh: () => void }) {

  const { startShuriken, stopShuriken } = useShuriken();
  
  const toggleShuriken = async (shuriken: Shuriken) => {
    if (shuriken.shuriken.type.type !== "Daemon") return
    if (shuriken.shuriken.status === "running") {
      await stopShuriken(shuriken.shuriken.name)
    } else {
      await startShuriken(shuriken.shuriken.name)
    }
  }

  useEffect(() => {
    console.log(shurikens)
  }, [shurikens])

  return (
    <div className="space-y-4 md:space-y-6">
      {/* Shurikens Section */}
      <div>
        <div className="flex justify-between">
          <h2 className="text-xl font-semibold mb-3 px-1">Shurikens</h2>
          <Button size={"icon"} onClick={onRefresh}>
            <RefreshCcw />
          </Button>
        </div>
        {shurikens.length !== 0 && shurikens !== undefined ? (
          <div>
            {gridView === "grid" ? (
              <div className="grid grid-cols-2 sm:grid-cols-2 gap-3 md:gap-4">
                {shurikens.map((service) => (
                  <Card key={service.shuriken.name} className="bg-card border-border py-0">
                    <CardHeader className="p-3 md:p-4 pb-0 md:pb-2 flex-row items-center justify-between space-y-0">
                      <div className="flex items-center gap-1">
                        <div className={`p-1.5 rounded-md mr-4 bg-muted ${service.shuriken.status === "running" ? "bg-green-500" : "bg-primary/10"}`} />
                        <CardTitle className="text-sm md:text-base">{service.shuriken.name}</CardTitle>
                      </div>
                    </CardHeader>
                    <CardFooter className={`h-full p-3 pr-2 md:p-4 ${service.shuriken.type.type === "Daemon" ? "pt-0" : "mt-4"} flex gap-2`}>
                      <Button
                        variant={service.shuriken.type.type === "Daemon" ? (service.shuriken.status === "running" ? "destructive" : "default") : "outline"}
                        className="text-xs md:text-sm h-8 px-0"
                        style={{ width: "90%" }}
                        onClick={() => toggleShuriken(service)}
                      >
                        {service.shuriken.type.type === "Daemon" ? (service.shuriken.status === "running" ? "Stop" : "Start") : "Manage"}
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
                    <TableRow key={service.shuriken.name}>
                      <TableCell className="text-center">{service.shuriken.name}</TableCell>
                      <TableCell className="text-center">{service.shuriken.status}</TableCell>
                      <TableCell className="text-center">{service.shuriken.type.type === "Daemon"? "Daemon": "Executable"}</TableCell>
                      <TableCell className="flex justify-center">
                        <Button
                          variant={service.shuriken.type.type === "Daemon" ? (service.shuriken.status === "running" ? "destructive" : "default") : "outline"}
                          style={{ width: "40%" }}
                          onClick={() => toggleShuriken(service)}
                        >
                          {service.shuriken.type.type === "Daemon" ? (service.shuriken.status === "running" ? "Stop" : "Start") : "Manage"}
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </div>
        ) : (
          <div className="text-center text-muted-foreground my-8">
            No Shurikens found.
          </div>
        )}

        {/* Local Projects */}
        <Card className="bg-card border-border py-0 mt-4">
          <CardHeader className="p-3 md:p-4 pb-0 md:pb-2">
            <div className="flex items-center justify-between">
              <div>
                <CardTitle className="text-base md:text-lg">Local Projects</CardTitle>
                <CardDescription className="text-xs md:text-sm">Your web projects in htdocs directory</CardDescription>
              </div>
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