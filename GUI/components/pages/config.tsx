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
    devMode: false,
    serverurl: "https://ninja-rs.vercel.app",
    checkUpdates: true,
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
                    serverurl: e.target.value,
                    checkUpdates: config.checkUpdates,
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
                      serverurl: config.serverurl,
                      checkUpdates: e,
                      devMode: config.devMode
                    })
                  }} checked={config.checkUpdates} />
                </div>
              </div>
            </motion.div>

        </TabsContent>
      </Tabs>
    </div>
  )
}
