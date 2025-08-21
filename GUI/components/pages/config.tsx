"use client"

import { Dispatch, SetStateAction, useState } from "react"
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/components/ui/select"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Input } from "../ui/input"
import { Button } from "../ui/button"
import { SaveIcon } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"

const tabs = ["Ninja"]

export default function Configuration({configtmp, setConfigtmp}: {configtmp?: NinjaConfig, setConfigtmp?: Dispatch<SetStateAction<NinjaConfig>>}) {
  const [config, setConfig] = useState<NinjaConfig>({
    mcp: {
      enabled: false,
      transport: "stdio",
      hostname: "localhost",
      port: 8080,
    },
    devMode: false,
    serverurl: "https://ninja-rs.vercel.app",
    checkUpdates: true,
    backups: {
      enabled:true,
      path:"./backups",
      schedule:"manual"
    }
  })

  return (
    <div className="space-y-2">
      <Tabs defaultValue="ninja" className="w-full">
        <div className="flex justify-between">

        <TabsList className={`grid grid-cols-${tabs.length} max-w-md mb-2`}>
          {tabs.map((tab, index) => (
            <TabsTrigger value={tab.toLowerCase()} key={index} className="text-sm font-medium">
              {tab}
            </TabsTrigger>))}
        </TabsList>
            <Button>
              <SaveIcon className="mr-2 h-4 w-4" />
              Save Configuration
            </Button>
        </div>
        <TabsContent value="ninja">
          <AnimatePresence>
            <motion.div
              className="w-full p-4 bg-muted rounded-md"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
            >
              <div className="flex items-center justify-between w-full px-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">Enable MCP</span>
                </div>

                <div className="flex items-center gap-2">
                  <Switch
                    id="mcp-switch"
                    onCheckedChange={(e) =>
                      setConfig({
                        mcp: {
                          enabled: e,
                          transport: config.mcp.transport,
                          hostname: config.mcp.hostname || "localhost",
                          port: config.mcp.port || 8080,
                        },
                        serverurl: config.serverurl,
                        checkUpdates: config.checkUpdates,
                        backups: config.backups,
                        devMode: config.devMode
                      })
                    }

                    checked={config.mcp.enabled}
                  />
                </div>
              </div>
                  
              {/* Animate the config fields when MCP is enabled */}
              {config.mcp.enabled && (
                <motion.div
                  key="mcp-options"
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 1, height: 0 }}
                  transition={{ duration: 0.2 }}
                  className="overflow-hidden"
                >
                  <motion.div className="space-y-2 flex justify-between mt-4">
                    <Label htmlFor="mcp-transport" className="text-sm font-medium">
                      Transport
                    </Label>
                    <div className="flex items-center gap-2">
                      <Label
                        className={`${
                          config.mcp.transport === "stdio"
                            ? "text-foreground"
                            : "text-foreground/50"
                        }`}
                      >
                        STDIO
                      </Label>
                      <Switch
                        id="mcp-transport"
                        onCheckedChange={(e) =>
                          setConfig({
                            mcp: {
                              ...config.mcp,
                              transport: e ? "http" : "stdio",
                            },
                            serverurl: config.serverurl,
                            checkUpdates: config.checkUpdates,
                            backups: config.backups,
                            devMode: config.devMode
                          })
                        }
                      />
                      <Label
                        className={`${
                          config.mcp.transport === "http"
                            ? "text-foreground"
                            : "text-foreground/50"
                        }`}
                      >
                        HTTP
                      </Label>
                    </div>
                  </motion.div>
                      
                  <motion.div
                    className={`${
                      config.mcp.transport === "http"
                        ? "text-foreground"
                        : "text-foreground/50"
                    }`}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 10 }}
                    transition={{ duration: 0.2 }}
                  >
                    <h2 className="p-2 py-4 text-lg">Transport options</h2>
                    <div className="flex justify-between ml-2 mt-2">
                      <p>Port</p>
                      <Input
                        type="number"
                        placeholder="8080"
                        className="w-48"
                        min={1025}
                        max={65535}
                        disabled={config.mcp.transport !== "http"}
                      />
                    </div>
                  </motion.div>
                </motion.div>
              )}
            </motion.div>
          </AnimatePresence>
          
          <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
              className="w-full p-4 bg-muted rounded-md mt-4"
            >
              <div className="flex items-center justify-between w-full px-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">Custom server</span>
                </div>

                <div className="flex items-center gap-2">
                  <Input onChange={(e) => setConfig({
                    mcp: config.mcp,
                    serverurl: e.target.value,
                    checkUpdates: config.checkUpdates,
                    backups: config.backups,
                    devMode: config.devMode
                  })} value={config.serverurl} type="url" />
                </div>
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
              className="w-full p-4 bg-muted rounded-md mt-4"
            >
              <div className="flex items-center justify-between w-full px-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">Check for updates</span>
                </div>

                <div className="flex items-center gap-2">
                  <Switch onCheckedChange={(e) => {
                    setConfig({
                      mcp:config.mcp,
                      serverurl: config.serverurl,
                      checkUpdates: e,
                      backups: config.backups,
                      devMode: config.devMode
                    })
                  }} checked={config.checkUpdates} />
                </div>
              </div>
            </motion.div>

            <AnimatePresence>
            <motion.div
              className="w-full p-4 bg-muted rounded-md mt-4"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
            >
              <div className="flex items-center justify-between w-full px-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">Enable backups</span>
                </div>

                <div className="flex items-center gap-2">
                  <Switch
                    id="backup-switch"
                    onCheckedChange={(e) =>
                      setConfig({
                        mcp: config.mcp,
                        serverurl: config.serverurl,
                        checkUpdates: config.checkUpdates,
                        backups: {
                          enabled: e,
                          path: config.backups.path,
                          schedule: config.backups.schedule
                        },
                        devMode: config.devMode
                      })
                    }

                    checked={config.backups.enabled}
                  />
                </div>
              </div>
                  
              {/* Animate the config fields when MCP is enabled */}
              {config.backups.enabled && (
                <motion.div
                  key="backup-options"
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 1, height: 0 }}
                  transition={{ duration: 0.2 }}
                  className="overflow-hidden"
                >
                  <motion.div className="space-y-2 flex justify-between mt-4">
                    <Label htmlFor="backup-transport" className="text-sm ml-4 font-medium">
                      Backup Options
                    </Label>
                  </motion.div>
                      
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 10 }}
                    transition={{ duration: 0.2 }}
                  >
                    <div className="flex justify-between ml-4 mt-4">
                      <p>Path</p>
                      <Input
                      className="w-fit mb-2 mr-2"
                        onChange={(e) => {
                          setConfig({
                            mcp: config.mcp,
                            serverurl: config.serverurl,
                            checkUpdates: config.checkUpdates,
                            backups: {
                              enabled: config.backups.enabled,
                              path: e.target.value,
                              schedule: config.backups.schedule
                            },
                            devMode: config.devMode
                          })
                        }}
                        value={config.backups.path}
                      />
                    </div>

                    <div className="flex justify-between ml-4 mt-2">
                      <p>Backup Schedule</p>
                      <Select>
                        <SelectTrigger className="w-[180px]">
                          <SelectValue placeholder="Schedule" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="daily">Daily</SelectItem>
                          <SelectItem value="weekly">Weekly</SelectItem>
                          <SelectItem value="manual">Manual</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </motion.div>
                </motion.div>
              )}
            </motion.div>
          </AnimatePresence>

        </TabsContent>
      </Tabs>
    </div>
  )
}
