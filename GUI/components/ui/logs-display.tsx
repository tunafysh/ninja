// components/LogsDisplay.tsx
import { useEffect, useRef, useState } from "react";
import { readTextFile, watch } from "@tauri-apps/plugin-fs";
import { resolve } from "@tauri-apps/api/path";
import clsx from "clsx";
import { Shuriken } from "@/lib/types";

type LogEntry = {
  timestamp: string;
  level: "info" | "warn" | "error";
  message: string;
};

// Parse lines like: [2025-10-09 13:00:00] [INFO] Message
const parseLogLine = (line: string): LogEntry | null => {
  const regex = /^\[(.+?)\]\s+\[(\w+)\]\s+(.+)$/;
  const match = line.match(regex);
  if (!match) return null;

  const [, timestamp, levelRaw, message] = match;
  const level = levelRaw.toLowerCase() as LogEntry["level"];
  if (!["info", "warn", "error"].includes(level)) return null;

  return { timestamp, level, message };
};

export const LogsDisplay = ({shuriken}: {shuriken: Shuriken} ) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const lastLength = useRef(0);

  useEffect(() => {
    const startWatching = async () => {
      const logFilePath = await resolve(`shurikens/${shuriken.shuriken.name}/${shuriken.logs?.log_path}`); // adjust to your path

      // Initial read
      const content = await readTextFile(logFilePath);
      const initialLogs = content
        .split("\n")
        .map(parseLogLine)
        .filter(Boolean) as LogEntry[];
      setLogs(initialLogs);
      lastLength.current = content.length;

      // Watch for file changes
      const unwatch = await watch(logFilePath, async () => {
        const updatedContent = await readTextFile(logFilePath);

        // Append only new content
        if (updatedContent.length > lastLength.current) {
          const diff = updatedContent.slice(lastLength.current);
          const newLines = diff
            .split("\n")
            .map(parseLogLine)
            .filter(Boolean) as LogEntry[];

          if (newLines.length > 0) {
            setLogs((prev) => [...prev, ...newLines]);
          }
        }

        lastLength.current = updatedContent.length;
      });

      return () => {
        unwatch();
      };
    };

    startWatching();
  }, []);

  // Auto-scroll on new logs
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const getLogStyle = (level: LogEntry["level"]) => {
    switch (level) {
      case "error":
        return "text-red-500";
      case "warn":
        return "text-yellow-500";
      case "info":
      default:
        return "text-gray-300";
    }
  };

  return (
    <div
      ref={containerRef}
      className="bg-gray-900 p-4 h-96 overflow-y-auto font-mono text-sm rounded"
    >
      {logs.length === 0 ? (
        <p className="text-gray-500 italic">No logs yet...</p>
      ) : (
        logs.map((log, idx) => (
          <div key={idx} className={clsx(getLogStyle(log.level))}>
            [{log.timestamp}] [{log.level.toUpperCase()}] {log.message}
          </div>
        ))
      )}
    </div>
  );
};
