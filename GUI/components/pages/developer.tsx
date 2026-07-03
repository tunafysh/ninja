"use client";

import { useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { readTextFile, BaseDirectory } from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";

type LogEntry = {
  id: string;
  text: string;
  type: "command" | "system";
};

export default function DeveloperModePanel() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [command, setCommand] = useState("");

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const lastLineCountRef = useRef<number>(0);
  const isFirstLoad = useRef<boolean>(true);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  useEffect(() => {
    const fetchLogs = async () => {
      try {
        const content = await readTextFile("logs/Ninja.log", {
          baseDir: BaseDirectory.AppData,
        });

        const lines = content.split("\n").filter(Boolean);
        
        if (isFirstLoad.current) {
          const initialLogs = lines.map((line, idx) => ({
            id: `file-init-${idx}`,
            text: line,
            type: "system" as const,
          }));
          setLogs(initialLogs);
          lastLineCountRef.current = lines.length;
          isFirstLoad.current = false;
        } else if (lines.length > lastLineCountRef.current) {
          const newLines = lines.slice(lastLineCountRef.current);
          const newEntries = newLines.map((line, idx) => ({
            id: `file-append-${Date.now()}-${idx}`,
            text: line,
            type: "system" as const,
          }));

          setLogs((prev) => [...prev, ...newEntries]);
          lastLineCountRef.current = lines.length;
        }
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

    const cmd = command;
    setCommand("");

    const commandEntry: LogEntry = {
      id: `cmd-${Date.now()}`,
      text: `> ${cmd}`,
      type: "command",
    };
    setLogs((prev) => [...prev, commandEntry]);

    try {
      const results: string[] = await invoke("execute_dsl", { command: cmd });
      
      const responseEntries = results.join("\n").split("\n").filter(Boolean).map((line, idx) => ({
        id: `cmd-res-${Date.now()}-${idx}`,
        text: line,
        type: "command" as const,
      }));

      setLogs((prev) => [...prev, ...responseEntries]);
    } catch (e) {
      setLogs((prev) => [
        ...prev,
        {
          id: `cmd-err-${Date.now()}`,
          text: `[ERROR] ${String(e)}`, // Prefixed with [ERROR] for automatic color coding
          type: "command",
        },
      ]);
    }
  };

  // Helper function to return colors based on log level content
  const getLogColor = (log: LogEntry) => {
    if (log.type === "command" && log.text.startsWith(">")) {
      return "text-cyan-400 font-semibold"; // User inputs
    }

    const lowerText = log.text.toLowerCase();
    
    if (lowerText.includes("error") || lowerText.includes("[err]")) {
      return "text-destructive font-medium"; // Soft red (using shadcn theme variable)
    }
    if (lowerText.includes("warn")) {
      return "text-yellow-500 font-medium"; // Amber/Yellow
    }
    if (lowerText.includes("info")) {
      return "text-blue-400"; // Blue
    }
    if (lowerText.includes("debug")) {
      return "text-muted-foreground/70 text-xs"; // Muted gray/smaller for less noise
    }

    return "text-foreground"; // Fallback color
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") executeCommand();
  };

  return (
  // 1. Give the main outer wrapper a fixed viewport height and make it a flex column
  <div className="w-full max-w-4xl mx-auto h-[calc(100vh-7rem)] flex flex-col pb-6">
    <h1 className="text-xl font-bold mb-4 shrink-0">Developer mode</h1>

    {/* 2. Tell the ScrollArea to grow (flex-1) but never push past its bounds (min-h-0) */}
    <ScrollArea className="flex-1 min-h-0 mb-4 border rounded p-4 bg-muted font-mono text-sm">
      <div className="space-y-1">
        {logs.map((log) => (
          <div
            key={log.id}
            className={`whitespace-pre-wrap transition-colors ${getLogColor(log)}`}
          >
            {log.text}
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>
    </ScrollArea>

    {/* 3. Keep the input bar at its native size at the bottom */}
    <div className="flex gap-2 shrink-0">
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