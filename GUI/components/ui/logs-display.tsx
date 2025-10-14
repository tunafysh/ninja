// components/LogsDisplay.tsx
import { useEffect, useRef, useState } from "react";
import { readTextFile, watch } from "@tauri-apps/plugin-fs";
import { resolve } from "@tauri-apps/api/path";
import clsx from "clsx";
import { Shuriken } from "@/lib/types";
import { ChevronDown, ChevronRight } from "lucide-react";

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

export const LogsDisplay = ({ shuriken }: { shuriken: Shuriken }) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [collapsed, setCollapsed] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const lastLength = useRef(0);

  useEffect(() => {
    const startWatching = async () => {
      const logFilePath = await resolve(
        `shurikens/${shuriken.metadata.name}/${shuriken.logs?.log_path}`
      );

      const content = await readTextFile(logFilePath);
      const initialLogs = content
        .split("\n")
        .map(parseLogLine)
        .filter(Boolean) as LogEntry[];
      setLogs(initialLogs);
      lastLength.current = content.length;

      const unwatch = await watch(logFilePath, async () => {
        const updatedContent = await readTextFile(logFilePath);

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

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const getLogStyle = (level: LogEntry["level"]) => {
    switch (level) {
      case "error":
        return "text-red-400 border-l-2 border-red-600 bg-red-950/10";
      case "warn":
        return "text-yellow-400 border-l-2 border-yellow-600 bg-yellow-950/10";
      case "info":
      default:
        return "text-gray-300 border-l-2 border-gray-700 bg-gray-900/30";
    }
  };

  const getLevelBadge = (level: LogEntry["level"]) => {
    const base = "px-2 py-0.5 rounded text-xs font-semibold uppercase";
    switch (level) {
      case "error":
        return `${base} bg-red-900 text-red-300`;
      case "warn":
        return `${base} bg-yellow-900 text-yellow-300`;
      case "info":
      default:
        return `${base} bg-gray-800 text-gray-300`;
    }
  };

  return (
    <div className="rounded-lg border border-gray-700 overflow-hidden bg-black/60">
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-2 border-b border-gray-700 bg-gray-900/70 cursor-pointer select-none"
        onClick={() => setCollapsed((prev) => !prev)}
      >
        <div className="flex items-center gap-2 text-gray-300 font-semibold text-sm">
          {collapsed ? (
            <ChevronRight size={16} className="text-gray-400" />
          ) : (
            <ChevronDown size={16} className="text-gray-400" />
          )}
          Logs for <span className="text-blue-400">{shuriken.metadata.name}</span>
        </div>
        <div className="text-xs text-gray-500">
          {logs.length} {logs.length === 1 ? "entry" : "entries"}
        </div>
      </div>

      {!collapsed && (
        <div
          ref={containerRef}
          className="h-96 overflow-y-auto font-mono text-sm p-2 bg-gray-950"
        >
          {logs.length === 0 ? (
            <p className="text-gray-600 italic text-center py-8">
              No logs yet...
            </p>
          ) : (
            logs.map((log, idx) => (
              <div
                key={idx}
                className={clsx(
                  "flex items-start gap-3 px-3 py-1.5 border-b border-gray-800/60 hover:bg-gray-800/30 transition-colors",
                  getLogStyle(log.level)
                )}
              >
                <span className="text-gray-500 text-xs whitespace-nowrap min-w-[125px]">
                  {log.timestamp}
                </span>
                <span className={getLevelBadge(log.level)}>{log.level}</span>
                <span className="flex-1 whitespace-pre-wrap break-words">
                  {log.message}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
