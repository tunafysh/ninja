"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { Database, FileCode, FolderOpen, RefreshCw, Search, Shield, Terminal, Trash2, Upload } from "lucide-react"

export default function Tools() {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <Card className="bg-card border-border">
          <CardHeader className="pb-2">
            <CardTitle className="text-lg flex items-center gap-2">
              <Database className="h-5 w-5 text-primary" />
              Database Tools
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button variant="outline" className="w-full justify-start">
              <Database className="mr-2 h-4 w-4" />
              phpMyAdmin
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <RefreshCw className="mr-2 h-4 w-4" />
              MySQL Console
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <Shield className="mr-2 h-4 w-4" />
              MySQL Security
            </Button>
          </CardContent>
        </Card>

        <Card className="bg-card border-border">
          <CardHeader className="pb-2">
            <CardTitle className="text-lg flex items-center gap-2">
              <FileCode className="h-5 w-5 text-primary" />
              PHP Tools
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button variant="outline" className="w-full justify-start">
              <FileCode className="mr-2 h-4 w-4" />
              PHP Info
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <Terminal className="mr-2 h-4 w-4" />
              PHP Console
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <Search className="mr-2 h-4 w-4" />
              Extension Manager
            </Button>
          </CardContent>
        </Card>

        <Card className="bg-card border-border">
          <CardHeader className="pb-2">
            <CardTitle className="text-lg flex items-center gap-2">
              <FolderOpen className="h-5 w-5 text-primary" />
              File Tools
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button variant="outline" className="w-full justify-start">
              <FolderOpen className="mr-2 h-4 w-4" />
              File Explorer
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <Upload className="mr-2 h-4 w-4" />
              File Upload
            </Button>
            <Button variant="outline" className="w-full justify-start">
              <Trash2 className="mr-2 h-4 w-4" />
              Clean Temp Files
            </Button>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="sql" className="w-full">
        <TabsList className="grid grid-cols-2 max-w-md mb-6">
          <TabsTrigger value="sql">SQL Query</TabsTrigger>
          <TabsTrigger value="backup">Backup</TabsTrigger>
        </TabsList>

        <TabsContent value="sql">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>SQL Query Tool</CardTitle>
              <CardDescription>Execute SQL queries directly on your database</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="database-select">Select Database</Label>
                <div className="flex gap-2">
                  <Input id="database-select" defaultValue="mysql" className="bg-muted" />
                  <Button variant="outline">Connect</Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="sql-query">SQL Query</Label>
                <Textarea
                  id="sql-query"
                  rows={6}
                  className="font-mono text-sm bg-muted border-slate-700"
                  defaultValue="SELECT * FROM users LIMIT 10;"
                />
              </div>

              <Button>Execute Query</Button>

              <div className="space-y-2">
                <Label>Results</Label>
                <div className="bg-slate-900 border border-slate-700 rounded-md p-4 h-[200px] overflow-auto">
                  <p className="text-slate-400 text-sm">Query results will appear here...</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="backup">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>Backup & Restore</CardTitle>
              <CardDescription>Create and restore backups of your databases and files</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-4">
                  <h3 className="text-lg font-medium">Database Backup</h3>

                  <div className="space-y-2">
                    <Label htmlFor="backup-database">Select Database</Label>
                    <Input id="backup-database" defaultValue="all_databases" className="bg-muted" />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="backup-location">Backup Location</Label>
                    <Input id="backup-location" defaultValue="C:/ninja/mysql/backup" className="bg-muted" />
                  </div>

                  <Button className="w-full">Create Database Backup</Button>
                </div>

                <div className="space-y-4">
                  <h3 className="text-lg font-medium">File Backup</h3>

                  <div className="space-y-2">
                    <Label htmlFor="backup-directory">Select Directory</Label>
                    <Input id="backup-directory" defaultValue="C:/ninja/htdocs" className="bg-muted" />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="backup-file-location">Backup Location</Label>
                    <Input id="backup-file-location" defaultValue="C:/ninja/backup" className="bg-muted" />
                  </div>

                  <Button className="w-full">Create File Backup</Button>
                </div>
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
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
