"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from "@tauri-apps/api/core"
import { useState } from "react"

export default function Backup() {
  const [restoringFile, setRestoringFile] = useState("")

  async function handleBackupNow() {
    try {
      const path = await invoke<string>("backup_now")
      toast.success(`Backup created: ${path}`)
    } catch {
      toast.error("Failed to create backup.")
    }
  }

  async function handleRestore() {
    try {
      await invoke("restore_backup", { file: restoringFile })
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
