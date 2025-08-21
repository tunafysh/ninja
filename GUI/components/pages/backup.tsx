"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

export default function Tools() {
  return (
    <div className="space-y-6">  
      <Card className="bg-card border-border">
        <CardHeader>
          <CardTitle>Backup & Restore</CardTitle>
          <CardDescription>Create and restore backups of your shurikens and projects</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-4">
              <h3 className="text-lg font-medium">File Backup</h3>
              <div className="space-y-2">
                <Label htmlFor="backup-directory">Select Directory</Label>
                <Input id="backup-directory" defaultValue="C:/ninja/htdocs" className="bg-muted" />
              </div>
              <Button className="w-full">Create File Backup</Button>
            </div>
          <div className="space-y-4">
            <h3 className="text-lg font-medium">Restore Backup</h3>
            <div className="space-y-2">
              <Label htmlFor="restore-file">Select Backup File</Label>
              <div className="flex gap-2">
                <Input id="restore-file" className="bg-muted" />
                <Button variant="outline">Browse</Button>
              </div>
            </div>
            <Button>Restore Selected Backup</Button>
          </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
