"use client"

import { useEffect, useState } from "react"
import { Card, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { MoreHorizontal, RefreshCcw, FolderOpen } from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from "@/components/ui/dropdown-menu"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useShuriken } from "@/hooks/use-shuriken"
import { invoke } from "@tauri-apps/api/core"
import LocalProjectsSidebar from "../ui/projects-pane"

export default function Dashboard({ gridView }: { gridView: "grid" | "list" }) {
  const { allShurikens, refreshShurikens, startShuriken, stopShuriken, loading } = useShuriken()
  const [projects, setProjects] = useState<string[]>([""])

  const refreshProjects = () => {
    invoke<string[]>("get_projects").then((e) => {
      setProjects(e)
      console.log("Projects found: " + e)
    })
  }

  const toggleShuriken = async (shuriken: typeof allShurikens[number]) => {
    if (shuriken.metadata.type !== "daemon") return
    if (shuriken.status === "running") {
      await stopShuriken(shuriken.metadata.name)
    } else {
      await startShuriken(shuriken.metadata.name)
    }
  }

  const openShurikensFolder = async () => {
    await invoke("open_dir", { path: "shurikens" })
  }

  const openProjectsFolder = async () => {
    await invoke("open_dir", { path: "projects" })
  }

  const openSpecificProject = async (name: string) => {
    await invoke("open_dir", { path: "projects/" + name })
  }

  useEffect(() => {
    refreshShurikens()
    refreshProjects()
  }, [refreshShurikens])

  return (
    <div className="space-y-4 md:space-y-6 select-none">
        {/* Shurikens Section */}
        <div className="p-3 md:p-4">
          <div className="flex justify-between">
            <h2 className="text-xl font-semibold mb-3 px-1">Shurikens</h2>
            <div className="flex gap-3">
              <Button
                size="icon"
                variant="ghost"
                className="hover:bg-accent rounded-lg"
                onClick={refreshShurikens}
              >
                <RefreshCcw className="h-4 w-4" />
              </Button>
              <Button size="icon" variant={"outline"} onClick={openShurikensFolder}>
                <FolderOpen />
              </Button>
            </div>
          </div>

          {loading ? (
            <p className="text-center my-8">Loading shurikens...</p>
          ) : allShurikens.length > 0 ? (
            gridView === "grid" ? (
              <div className="grid lg:grid-cols-3 md:grid-cols-2 sm:grid-cols-1 gap-3 md:gap-4">
                {allShurikens.map((service) => (
                  <Card key={service.metadata.name} className="bg-card border-border py-0">
                    <CardHeader className="p-3 md:p-4 pb-0 md:pb-2 flex-row items-center justify-between space-y-0">
                      <div className="flex items-center gap-1">
                        <div
                          className={`p-1.5 rounded-md mr-2 ${
                            service.status === "running" ? "bg-green-500" : "bg-muted"
                          }`}
                        />
                        <CardTitle className="text-sm md:text-base flex gap-2">
                          <p className="mr-2">{service.metadata.name}</p>
                          <Badge>{service.metadata.version}</Badge>
                        </CardTitle>
                      </div>
                    </CardHeader>
                    <CardFooter className="pb-4 gap-2">
                      <Button
                        variant={
                          service.metadata.type === "daemon"
                            ? service.status === "running"
                              ? "destructive"
                              : "default"
                            : "outline"
                        }
                        className="text-xs md:text-sm h-8 px-0 w-[86%]"
                        onClick={() => toggleShuriken(service)}
                      >
                        {service.metadata.type === "daemon"
                          ? service.status === "running"
                            ? "Stop"
                            : "Start"
                          : "Manage"}
                      </Button>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="outline" size="sm" className="px-2 h-8">
                            <MoreHorizontal className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() =>{
                              stopShuriken(service.metadata.name) 
                              startShuriken(service.metadata.name)
                            }}
                          >
                            Restart
                          </DropdownMenuItem>
                          <DropdownMenuItem>View Logs</DropdownMenuItem>
                          <DropdownMenuItem>Configure</DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </CardFooter>
                  </Card>
                ))}
              </div>
            ) : (
              <Table className="border-border border-2 my-4 rounded-lg">
                <TableHeader>
                  <TableRow>
                    <TableHead className="text-center">Shuriken</TableHead>
                    <TableHead className="text-center">Status</TableHead>
                    <TableHead className="text-center">Maintenance</TableHead>
                    <TableHead className="text-center">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {allShurikens.map((service) => (
                    <TableRow key={service.metadata.name}>
                      <TableCell className="text-center">{service.metadata.name}</TableCell>
                      <TableCell className="text-center">{service.status}</TableCell>
                      <TableCell className="text-center">
                        {service.metadata.type === "daemon" ? "daemon" : "executable"}
                      </TableCell>
                      <TableCell className="flex justify-center">
                        <Button
                          variant={
                            service.metadata.type === "daemon"
                              ? service.status === "running"
                                ? "destructive"
                                : "default"
                              : "outline"
                          }
                          style={{ width: "40%" }}
                          onClick={() => toggleShuriken(service)}
                        >
                          {service.metadata.type === "daemon"
                            ? service.status === "running"
                              ? "Stop"
                              : "Start"
                            : "Manage"}
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )
          ) : (
            <div className="text-center text-muted-foreground my-8">No Shurikens found.</div>
          )}
        </div>

      {/* Local Projects */}
      <LocalProjectsSidebar
        projects={projects}
        refreshProjects={refreshProjects}
        openProjectsFolder={openProjectsFolder}
        openSpecificProject={openSpecificProject}
      />
    </div>
  )
}
