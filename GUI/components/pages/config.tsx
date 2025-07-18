"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Server, Database, FileCode, Save } from "lucide-react"
import { ModeToggle } from "../ui/themetoggle"

const tabs = ["Ninja"]

export default function Configuration() {
  return (
    <div className="space-y-6">
      <Tabs defaultValue="ninja" className="w-full">
        <TabsList className={`grid grid-cols-${tabs.length} max-w-md mb-6`}>
          {tabs.map((tab, index) => (
            <TabsTrigger value={tab.toLowerCase()} key={index} className="text-sm font-medium">
              {tab}
            </TabsTrigger>))}
        </TabsList>

        <TabsContent value="ninja">
            <Card className="w-full">
              
            </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
