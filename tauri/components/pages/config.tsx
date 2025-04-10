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

export default function Configuration() {
  const [apachePort, setApachePort] = useState("80")
  const [mysqlPort, setMysqlPort] = useState("3306")

  return (
    <div className="space-y-6">
      <Tabs defaultValue="apache" className="w-full">
        <TabsList className="grid grid-cols-3 max-w-md mb-6">
          <TabsTrigger value="apache" className="flex items-center gap-2">
            <Server className="h-4 w-4" />
            Apache
          </TabsTrigger>
          <TabsTrigger value="mysql" className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            MySQL
          </TabsTrigger>
          <TabsTrigger value="php" className="flex items-center gap-2">
            <FileCode className="h-4 w-4" />
            PHP
          </TabsTrigger>
        </TabsList>

        <TabsContent value="apache">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>Apache Configuration</CardTitle>
              <CardDescription>Manage your Apache web server settings</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="apache-port">HTTP Port</Label>
                    <Input
                      id="apache-port"
                      value={apachePort}
                      onChange={(e) => setApachePort(e.target.value)}
                      className="bg-muted"
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="apache-ssl-port">SSL Port</Label>
                    <Input id="apache-ssl-port" defaultValue="443" className="bg-muted" />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="document-root">Document Root</Label>
                    <Input id="document-root" defaultValue="C:/xampp/htdocs" className="bg-muted" />
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-ssl">Enable SSL</Label>
                    <Switch id="enable-ssl" />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-gzip">Enable Gzip Compression</Label>
                    <Switch id="enable-gzip" defaultChecked />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-directory-listing">Directory Listing</Label>
                    <Switch id="enable-directory-listing" defaultChecked />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-htaccess">Allow .htaccess</Label>
                    <Switch id="enable-htaccess" defaultChecked />
                  </div>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="apache-config">httpd.conf</Label>
                <Textarea
                  id="apache-config"
                  rows={8}
                  className="font-mono text-sm bg-muted"
                  defaultValue={`# Apache configuration file
ServerRoot "C:/xampp/apache"
Listen ${apachePort}
LoadModule access_compat_module modules/mod_access_compat.so
LoadModule actions_module modules/mod_actions.so
LoadModule alias_module modules/mod_alias.so
# ... more configuration ...`}
                />
              </div>

              <div className="flex justify-end">
                <Button className="flex items-center gap-2">
                  <Save className="h-4 w-4" />
                  Save Configuration
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="mysql">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>MySQL Configuration</CardTitle>
              <CardDescription>Manage your MySQL database settings</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="mysql-port">MySQL Port</Label>
                    <Input
                      id="mysql-port"
                      value={mysqlPort}
                      onChange={(e) => setMysqlPort(e.target.value)}
                      className="bg-muted"
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="mysql-username">Default Username</Label>
                    <Input id="mysql-username" defaultValue="root" className="bg-muted" />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="mysql-password">Default Password</Label>
                    <Input id="mysql-password" type="password" className="bg-muted" />
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-remote">Allow Remote Connections</Label>
                    <Switch id="enable-remote" />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-strict-mode">Strict Mode</Label>
                    <Switch id="enable-strict-mode" defaultChecked />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="enable-innodb">Use InnoDB by Default</Label>
                    <Switch id="enable-innodb" defaultChecked />
                  </div>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="mysql-config">my.ini</Label>
                <Textarea
                  id="mysql-config"
                  rows={8}
                  className="font-mono text-sm bg-muted"
                  defaultValue={`# MySQL configuration file
[mysqld]
port=${mysqlPort}
socket=mysql
key_buffer_size=256M
max_allowed_packet=1M
table_open_cache=256
sort_buffer_size=1M
net_buffer_length=8K
read_buffer_size=1M
read_rnd_buffer_size=512K
myisam_sort_buffer_size=64M
# ... more configuration ...`}
                />
              </div>

              <div className="flex justify-end">
                <Button className="flex items-center gap-2">
                  <Save className="h-4 w-4" />
                  Save Configuration
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="php">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>PHP Configuration</CardTitle>
              <CardDescription>Manage your PHP settings and extensions</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="php-version">PHP Version</Label>
                    <Input id="php-version" defaultValue="8.2.0" className="bg-muted" readOnly />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="memory-limit">Memory Limit</Label>
                    <Input id="memory-limit" defaultValue="256M" className="bg-muted" />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="max-execution-time">Max Execution Time</Label>
                    <Input id="max-execution-time" defaultValue="30" className="bg-muted" />
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="display-errors">Display Errors</Label>
                    <Switch id="display-errors" />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="file-uploads">File Uploads</Label>
                    <Switch id="file-uploads" defaultChecked />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="allow-url-fopen">Allow URL fopen</Label>
                    <Switch id="allow-url-fopen" defaultChecked />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="short-open-tag">Short Open Tag</Label>
                    <Switch id="short-open-tag" />
                  </div>
                </div>
              </div>

              <div className="space-y-2">
                <Label>PHP Extensions</Label>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
                  {["mysqli", "pdo_mysql", "gd", "curl", "mbstring", "xml", "zip", "intl"].map((ext) => (
                    <div key={ext} className="flex items-center space-x-2">
                      <Switch id={`ext-${ext}`} defaultChecked />
                      <Label htmlFor={`ext-${ext}`}>{ext}</Label>
                    </div>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="php-config">php.ini</Label>
                <Textarea
                  id="php-config"
                  rows={8}
                  className="font-mono text-sm bg-muted"
                  defaultValue={`; PHP Configuration File
memory_limit = 256M
post_max_size = 8M
upload_max_filesize = 2M
max_execution_time = 30
display_errors = Off
error_reporting = E_ALL & ~E_DEPRECATED & ~E_STRICT
default_charset = "UTF-8"
; ... more configuration ...`}
                />
              </div>

              <div className="flex justify-end">
                <Button className="flex items-center gap-2">
                  <Save className="h-4 w-4" />
                  Save Configuration
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
