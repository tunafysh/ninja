"use client";

import { useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { readTextFile , BaseDirectory } from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core"

export default function DeveloperModePanel() {
  const [logs, setLogs] = useState<string[]>([]);
  const [command, setCommand] = useState("");
  const viewportRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    viewportRef.current?.scrollTo({ top: viewportRef.current.scrollHeight });
  }, [logs]);

  useEffect(() => {
    const fetchLogs = async () => {
      try {
        
        const content = await readTextFile("logs/Ninja.log", {
          baseDir: BaseDirectory.AppData,
        });

        setLogs(content.split("\n"));
      } catch (e) {
        console.error("Failed to read logs", e);
      }
    };

    fetchLogs();
    const interval = setInterval(fetchLogs, 2000);
    return () => clearInterval(interval);
  }, []);

  const executeCommand = async () => {
    if (!command.trim()) return;

    try {
      const results: string[] = await invoke("execute_dsl", { command });
      const output = results.join("\n"); // combine lines into one block

      setLogs(prev => [
        ...prev,
        `> ${command}`,
        output
      ]);

      setCommand("");
    } catch (e) {
      setLogs(prev => [...prev, `> ${command}`, `Error: ${e}`]);
      setCommand("");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") executeCommand();
  };

  return (
    <div className="w-full max-w-4xl mx-auto mt-6">
      <h1 className="text-xl font-bold mb-4">Developer mode</h1>
        <ScrollArea className="h-64 mb-4 border rounded p-2 bg-muted">
            {logs.map((log, idx) => (
              <div key={idx} className="font-mono text-sm" ref={viewportRef}>
                {log}
              </div>
            ))}
        </ScrollArea>

        <div className="flex gap-2">
          <Input
            className="flex-1 font-mono"
            placeholder="Enter DSL command..."
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          <Button onClick={executeCommand}>Run</Button>
        </div>
      </div>
  );
}
