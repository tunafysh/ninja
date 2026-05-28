"use client"

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { LogsDisplay } from "../ui/logs-display"
import { Shuriken } from "@/lib/types"
import { Suspense } from "react"

export default function Logs({ shurikens }: { shurikens: Shuriken[] }) {
  const withLogs = shurikens.filter(s => s.logs !== undefined)

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div className="space-y-1">
          <h2 className="text-xl font-bold tracking-tight">Server Logs</h2>
          <p className="text-muted-foreground">View and analyze your server logs</p>
        </div>
      </div>

      {withLogs.length > 0 ? (
        <Tabs
          className="w-full"
          defaultValue={withLogs[0].metadata.name} // <-- ensures first tab shows
        >
          <TabsList className={`grid grid-cols-${withLogs.length} max-w-md mb-6`}>
            {withLogs.map((value) => (
              <TabsTrigger
                key={value.metadata.name}
                value={value.metadata.name}
                className="flex items-center gap-2"
              >
                {value.metadata.name}
              </TabsTrigger>
            ))}
          </TabsList>

          {withLogs.map((value) => (
            <TabsContent
              key={value.metadata.name}
              value={value.metadata.name}
            >
              <Suspense fallback={<div className="p-4 text-gray-500">Loading logs...</div>}>
                <LogsDisplay shuriken={value} />
              </Suspense>
            </TabsContent>
          ))}
        </Tabs>
      ) : (
        <div className="flex w-full h-full items-center justify-center">
          <p>No log entry was defined for shurikens</p>
        </div>
      )}
    </div>
  )
}
