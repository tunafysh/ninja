"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import ProviderBox from "../ui/provider-box"
import { Server } from "lucide-react"
import { useState } from "react"
import Image from "next/image"
import GDrive from "@/public/drive.svg"
import OneDrive from "@/public/onedrive.svg"

export default function Backup() {
  const [providers, setProviders] = useState<boolean[]>([false, false, false])

  return (
    <div className="space-y-6 select-none">  
      <Card className="bg-card border-border">
        <CardHeader>
          <CardTitle>Backup & Restore</CardTitle>
          <CardDescription>Create and restore backups of your shurikens and projects</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-4">
              <h3 className="text-lg font-medium">Backup</h3>
                <h4 className="text-md font-medium">Select providers</h4>
              <div className="space-y-2 flex gap-4">
                <ProviderBox checked={providers[0]} name="Google drive" icon={<Image className="select-none" alt="Logo" width={60} height={60} src={GDrive} />} onCheck={() => setProviders([!providers[0], ...providers])}/>
                <ProviderBox checked={providers[1]} name="Onedrive" icon={<Image alt="Logo" width={35} height={35} src={OneDrive} />} onCheck={() => setProviders([...providers, !providers[1], ...providers])}/>
                <ProviderBox checked={providers[2]} name="Local" icon={<Server />} className="h-24" onCheck={() => setProviders([...providers, !providers[2]])}/>
              </div>
              <Button>Backup</Button>
            </div>
          <div className="space-y-4">
            <h3 className="text-lg font-medium">Restore</h3>
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
