"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Server, Database, FileCode, RefreshCw, Download } from "lucide-react"

export default function Logs() {
  const [apacheLog, setApacheLog] =
    useState(`[Sun Apr 09 16:40:12.123456 2023] [ssl:warn] [pid 123] AH01909: localhost:443:0 server certificate does NOT include an ID which matches the server name
[Sun Apr 09 16:40:12.234567 2023] [mpm_winnt:notice] [pid 123] AH00455: Apache/2.4.54 (Win64) OpenSSL/1.1.1p PHP/8.2.0 configured -- resuming normal operations
[Sun Apr 09 16:40:12.345678 2023] [mpm_winnt:notice] [pid 123] AH00456: Apache Lounge VS16 Server built: Jun 14 2022 13:22:10
[Sun Apr 09 16:40:12.456789 2023] [core:notice] [pid 123] AH00094: Command line: 'c:\\xampp\\apache\\bin\\httpd.exe -d C:/xampp/apache'
[Sun Apr 09 16:40:12.567890 2023] [mpm_winnt:notice] [pid 123] AH00418: Parent: Created child process 456
[Sun Apr 09 16:40:13.678901 2023] [ssl:warn] [pid 456] AH01909: localhost:443:0 server certificate does NOT include an ID which matches the server name
[Sun Apr 09 16:40:13.789012 2023] [mpm_winnt:notice] [pid 456] AH00354: Child: Starting 150 worker threads.
[Sun Apr 09 16:40:41.890123 2023] [php:notice] [pid 456] [client ::1:50778] PHP Notice: Undefined variable: foo in C:\\xampp\\htdocs\\test\\index.php on line 3`)

  const [mysqlLog, setMysqlLog] =
    useState(`2023-04-09T16:40:12.123456Z 0 [Note] [MY-000000] [MYSQL] C:\\xampp\\mysql\\bin\\mysqld.exe (mysqld 8.0.31) starting as process 789
2023-04-09T16:40:12.234567Z 1 [System] [MY-013576] [InnoDB] InnoDB initialization has started.
2023-04-09T16:40:12.345678Z 1 [System] [MY-013577] [InnoDB] InnoDB initialization has ended.
2023-04-09T16:40:12.456789Z 0 [System] [MY-011323] [Server] X Plugin ready for connections. Bind-address: '::' port: 33060, socket: C:/xampp/mysql/mysql.sock
2023-04-09T16:40:12.567890Z 0 [Warning] [MY-010068] [Server] CA certificate ca.pem is self signed.
2023-04-09T16:40:12.678901Z 0 [System] [MY-010931] [Server] C:\\xampp\\mysql\\bin\\mysqld.exe: ready for connections. Version: '8.0.31'  socket: ''  port: 3306  MySQL Community Server - GPL.
2023-04-09T16:40:12.789012Z 0 [System] [MY-013172] [Server] Received SHUTDOWN from user root. Shutting down mysqld (Version: 8.0.31).
2023-04-09T16:40:41.890123Z 0 [System] [MY-010910] [Server] C:\\xampp\\mysql\\bin\\mysqld.exe: Shutdown complete (mysqld 8.0.31)  MySQL Community Server - GPL.`)

  const [phpLog, setPhpLog] =
    useState(`[09-Apr-2023 16:40:12 UTC] PHP Warning:  mysqli_connect(): (HY000/1045): Access denied for user 'root'@'localhost' (using password: YES) in C:\\xampp\\htdocs\\test\\db.php on line 7
[09-Apr-2023 16:40:13 UTC] PHP Notice:  Undefined variable: name in C:\\xampp\\htdocs\\test\\index.php on line 12
[09-Apr-2023 16:40:14 UTC] PHP Warning:  file_get_contents(https://example.com/api): failed to open stream: HTTP request failed! in C:\\xampp\\htdocs\\test\\api.php on line 5
[09-Apr-2023 16:40:15 UTC] PHP Fatal error:  Uncaught Error: Call to undefined function missing_function() in C:\\xampp\\htdocs\\test\\functions.php:23
Stack trace:
#0 C:\\xampp\\htdocs\\test\\index.php(45): include()
#1 {main}
  thrown in C:\\xampp\\htdocs\\test\\functions.php on line 23
[09-Apr-2023 16:40:41 UTC] PHP Warning:  session_start(): Cannot start session when headers already sent in C:\\xampp\\htdocs\\test\\session.php on line 3`)

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div className="space-y-1">
          <h2 className="text-2xl font-bold tracking-tight">Server Logs</h2>
          <p className="text-muted-foreground">View and analyze your server logs</p>
        </div>
        <div className="flex items-center gap-2">
          <Select defaultValue="50">
            <SelectTrigger className="w-[120px]">
              <SelectValue placeholder="Lines" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="50">Last 50 lines</SelectItem>
              <SelectItem value="100">Last 100 lines</SelectItem>
              <SelectItem value="200">Last 200 lines</SelectItem>
              <SelectItem value="all">All lines</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" size="icon">
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="icon">
            <Download className="h-4 w-4" />
          </Button>
        </div>
      </div>

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
              <CardTitle>Apache Error Log</CardTitle>
              <CardDescription>httpd.error_log</CardDescription>
            </CardHeader>
            <CardContent>
              <pre className="bg-muted p-4 rounded-md text-xs font-mono h-[400px] overflow-auto whitespace-pre-wrap">
                {apacheLog}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="mysql">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>MySQL Error Log</CardTitle>
              <CardDescription>mysql.err</CardDescription>
            </CardHeader>
            <CardContent>
              <pre className="bg-muted p-4 rounded-md text-xs font-mono h-[400px] overflow-auto whitespace-pre-wrap">
                {mysqlLog}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="php">
          <Card className="bg-card border-border">
            <CardHeader>
              <CardTitle>PHP Error Log</CardTitle>
              <CardDescription>php_error_log</CardDescription>
            </CardHeader>
            <CardContent>
              <pre className="bg-muted p-4 rounded-md text-xs font-mono h-[400px] overflow-auto whitespace-pre-wrap">
                {phpLog}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
