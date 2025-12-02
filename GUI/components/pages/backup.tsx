"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectValue, SelectTrigger, SelectContent, SelectItem, SelectLabel } from "@/components/ui/select"
import { toast } from "sonner"
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from "@tauri-apps/api/core"
import { useState } from "react"

type CompressionType = "Fast" | "Normal" | "Best";

function parseCompressionType(s: string): CompressionType | undefined {
  if (s === "Fast" || s === "Normal" || s === "Best") {
    return s;
  }
  return undefined; // invalid value
}

export default function Backup() {
  const [restoringFile, setRestoringFile] = useState("")
  const [compressionType, setCompressionType] = useState<CompressionType>("Normal")
  async function handleBackupNow() {
    try {
      await invoke("backup_now", { level: compressionType })
      toast.success(`Backup created successfully.`)
    } catch(e) {
      toast.error(`Failed to create backup: ${e}`)
    }
  }

  async function handleRestore() {
    try {
      await invoke("backup_restore", { file: restoringFile })
      toast.success("Backup restored successfully")
    } catch {
      toast.error("Failed to restore backup.")
    }
  }

  return (
    <div className="select-none">
      <Card className="bg-background border-border shadow-sm">
        <CardHeader>
          <CardTitle className="text-xl">Backup & Restore</CardTitle>
          <CardDescription>
            Quickly create backups or restore existing archives
          </CardDescription>
        </CardHeader>

        <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-6">

          {/* BACKUP SECTION */}
          <div className="flex flex-col items-center justify-center p-6 space-y-4 bg-secondary/40 rounded-lg">
            <h3 className="text-lg font-medium">Backup Now</h3>
            <p className="text-sm text-muted-foreground text-center">
              Create a backup of your projects in a compressed <code>.tar.gz</code> archive.
            </p>
            <div className="flex w-full gap-2">
              <Select
                value={compressionType}
                onValueChange={(value) => setCompressionType(parseCompressionType(value) ?? compressionType)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select Compression Type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Fast">Fast</SelectItem>
                  <SelectItem value="Normal">Normal</SelectItem>
                  <SelectItem value="Best">Best</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button onClick={handleBackupNow} className="w-full md:w-auto">
              Backup Now (.tar.gz)
            </Button>
          </div>

          {/* RESTORE SECTION */}
          <div className="flex flex-col items-center justify-center p-6 space-y-4 bg-secondary/40 rounded-lg">
            <h3 className="text-lg font-medium">Restore Backup</h3>
            <p className="text-sm text-muted-foreground text-center">
              Select an existing backup archive to restore your projects.
            </p>
            <div className="flex w-full gap-2">
              <Input
                value={restoringFile}
                onChange={(e) => setRestoringFile(e.target.value)}
                placeholder="/path/to/backup.tar.gz"
                className="bg-muted border-border"
              />
              <Button
                variant="outline"
                onClick={async () => {
                  const path = await open({
                    filters: [{ name: 'Backup Files', extensions: ['tar.gz'] }]
                  })
                  if (path) setRestoringFile(path)
                }}
              >
                Browse
              </Button>
            </div>
            <Button onClick={handleRestore} className="w-full md:w-auto">
              Restore Selected Backup
            </Button>
          </div>

        </CardContent>
      </Card>
    </div>
  )
}
