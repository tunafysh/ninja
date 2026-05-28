"use client"

import { Dispatch, SetStateAction, useEffect, useState } from "react"
import { Card, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
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

export default function Dashboard({ gridView, setActiveIndex }: { gridView: "grid" | "list", setActiveIndex: Dispatch<SetStateAction<number>> }) {
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
    if (shuriken.state === "Running") {
      await stopShuriken(shuriken?.metadata?.name)
    } else {
      await startShuriken(shuriken?.metadata?.name)
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
                onClick={() => refreshShurikens()}
              >
                <RefreshCcw className="h-4 w-4" />
              </Button>
              <Button size="icon" variant={"outline"} onClick={() => openShurikensFolder()}>
                <FolderOpen />
              </Button>
            </div>
          </div>

          {loading ? (
            gridView === "grid" ? (
              <div className="grid lg:grid-cols-3 md:grid-cols-2 sm:grid-cols-1 gap-3 md:gap-4">
                {[...Array(4)].map((_, i) => (
                  <Card key={`skeleton-${i}`} className="bg-card border-border py-0">
                    <CardHeader className="p-3 md:p-4 pb-0 md:pb-2">
                      <div className="flex items-center gap-2">
                        <Skeleton className="h-6 w-6 rounded-md" />
                        <div className="flex-1 space-y-2">
                          <Skeleton className="h-4 w-24" />
                          <Skeleton className="h-4 w-12" />
                        </div>
                      </div>
                    </CardHeader>
                    <CardFooter className="pb-4 gap-2">
                      <Skeleton className="h-8 flex-1" />
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
                  {[...Array(4)].map((_, i) => (
                    <TableRow key={`skeleton-${i}`}>
                      <TableCell className="text-center"><Skeleton className="h-4 w-20 mx-auto" /></TableCell>
                      <TableCell className="text-center"><Skeleton className="h-4 w-16 mx-auto" /></TableCell>
                      <TableCell className="text-center"><Skeleton className="h-4 w-20 mx-auto" /></TableCell>
                      <TableCell className="text-center"><Skeleton className="h-8 w-20 mx-auto" /></TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )
          ) : allShurikens.length > 0 ? (
            gridView === "grid" ? (
              <div className="grid lg:grid-cols-3 md:grid-cols-2 sm:grid-cols-1 gap-3 md:gap-4">
                {allShurikens.map((service, index) => (
                  <Card key={service.metadata.name || `shuriken-${index}`} className="bg-card border-border py-0">
                    <CardHeader className="p-3 md:p-4 pb-0 md:pb-2 flex-row items-center justify-between space-y-0">
                      <div className="flex items-center gap-1">
                        <div
                          className={`p-1.5 rounded-md mr-2 ${
                            service.state === "Running" ? "bg-green-500" : "bg-muted"
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
                          service?.metadata?.type === "daemon"
                            ? service.state === "Running"
                              ? "destructive"
                              : "default"
                            : "outline"
                        }
                        className={`text-xs md:text-sm h-8 px-0 ${service.state === "Running"? "w-[86%]": "w-full"}`}
                        onClick={() => toggleShuriken(service)}
                      >
                        {service?.metadata?.type === "daemon"
                          ? service.state === "Running"
                            ? "Stop"
                            : "Start"
                          : "Manage"}
                      </Button>
                      {service.state === "Running" && (
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
                          <DropdownMenuItem
                          onClick={async () => {
                            const name = service?.metadata?.name
                            if (name) {
                              await invoke("lockpick_shuriken", { shuriken: name })
                            }
                          }}
                          >Lockpick</DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                      )}
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
                  {allShurikens.map((service, index) => (
                    <TableRow key={service?.metadata?.name || `shuriken-${index}`}>
                      <TableCell className="text-center">{service?.metadata?.name}</TableCell>
                      <TableCell className="text-center">{service.state.toString()}</TableCell>
                      <TableCell className="text-center">
                        {service?.metadata?.type === "daemon" ? "daemon" : "executable"}
                      </TableCell>
                      <TableCell className="flex justify-center">
                        <Button
                          variant={
                            service?.metadata?.type === "daemon"
                          ? service.state === "Running"
                                ? "destructive"
                                : "default"
                              : "outline"
                          }
                          style={{ width: "40%" }}
                          onClick={() => toggleShuriken(service)}
                        >
                          {service?.metadata?.type === "daemon"
                            ? service.state === "Running"
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
            <div className="text-center text-muted-foreground my-8">No Shurikens found. You can get shurikens at the <Button onClick={() => setActiveIndex(5)} variant={"link"} className="px-0.5">Armory</Button></div>
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
