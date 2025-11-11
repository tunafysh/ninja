"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Server, Database, FileCode, RefreshCw, Download } from "lucide-react"
import { LogsDisplay } from "../ui/logs-display"
import { Shuriken } from "@/lib/types"

export default function Logs({shurikens}: {shurikens: Shuriken[]}) {
  
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div className="space-y-1">
          <h2 className="text-xl font-bold tracking-tight">Server Logs</h2>
          <p className="text-muted-foreground">View and analyze your server logs</p>
        </div>
      </div>

      {shurikens.some(s => s.logs != null) ? (
        <Tabs className="w-full">
        <TabsList className="grid grid-cols-3 max-w-md mb-6">
          {shurikens.map((value, index) => value.logs!= null?(
          <TabsTrigger value={value.metadata.name} className="flex items-center gap-2">
            {value.metadata.name}
          </TabsTrigger>
          ): null)}          
        </TabsList>

      {shurikens.map((value, index) => value.logs!= null?(
        <TabsContent value={value.metadata.name}>
          <LogsDisplay shuriken={value} />
        </TabsContent>
      ): null)}
      </Tabs>
      ): (
        <div className="flex w-full h-full items-center justify-center">
          <p>No log entry was defined for shurikens</p>
        </div>
      )}

    </div>
  )
}
