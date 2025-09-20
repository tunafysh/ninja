import { useState, useRef } from "react";
import { CommandDialog, CommandInput, CommandList, CommandItem } from "@/components/ui/command";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";

export default function DslCommandPalette({ commandOpened, setCommandOpened }: { commandOpened: boolean; setCommandOpened: (open: boolean) => void }) {
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
    const input = inputRef.current!;
    if (e.key === "Enter") {
      const value = input.value.trim();
      if (!value) return;

      try {
        const result = await invoke<string>("execute_dsl", { command: value });
        toast.success(result);
      } catch (err) {
        toast.error(`Error: ${err}`);
      }

      setHistory((prev) => [...prev, value]);
      setHistoryIndex(null);
      input.value = "";
      setCommandOpened(false);
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      setHistoryIndex((prev) => {
        const newIndex = prev === null ? history.length - 1 : Math.max(prev - 1, 0);
        input.value = history[newIndex] ?? "";
        return newIndex;
      });
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHistoryIndex((prev) => {
        if (prev === null) return null;
        const newIndex = Math.min(prev + 1, history.length - 1);
        input.value = history[newIndex] ?? "";
        return newIndex;
      });
    }
  };

  return (
    <CommandDialog
      showCloseButton={true}
      open={commandOpened}
      onOpenChange={setCommandOpened}
      className="border shadow-md md:min-w-[450px]"
    >
      <CommandInput
        ref={inputRef}
        placeholder="Type a DSL command (e.g., select Apache; start;)"
        onKeyDown={handleKeyDown}
        className="rounded-b-none"
      />
      <CommandList className="rounded-t-none">
        {history.length === 0 ? (
          <CommandItem disabled>Enter a DSL command above</CommandItem>
        ) : (
          history.slice(-5).reverse().map((cmd, i) => (
            <CommandItem key={i} disabled className="text-sm opacity-60">
              {cmd}
            </CommandItem>
          ))
        )}
      </CommandList>
    </CommandDialog>
  );
}
