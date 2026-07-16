import { InstallProgress } from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export function useInstallShuriken() {
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    console.log("[Install] Hook mounted");

    let unlistenProgress: (() => void) | undefined;
    let unlistenStage: (() => void) | undefined;

    const init = async () => {
      console.log("[Install] Registering listeners...");

      unlistenProgress = await listen<number>("install-progress", (event) => {
        console.log("[Install] Progress event:", event.payload);

        setProgress((prev) => {
          const next = {
            progress: event.payload,
            stage: prev?.stage ?? "Starting...",
          };

          console.log("[Install] Updated progress state:", next);

          return next;
        });
      });

      console.log("[Install] Progress listener registered");

      unlistenStage = await listen<string>("install-stage", (event) => {
        console.log("[Install] Stage event:", event.payload);

        setProgress((prev) => {
          const next = {
            progress: prev?.progress ?? 0,
            stage: event.payload,
          };

          console.log("[Install] Updated stage state:", next);

          return next;
        });
      });

      console.log("[Install] Stage listener registered");
    };

    init().catch((err) => {
      console.error("[Install] Listener setup failed:", err);
    });

    return () => {
      console.log("[Install] Cleaning up listeners");

      unlistenProgress?.();
      unlistenStage?.();
    };
  }, []);

  const install = async (source: string) => {
    console.log("[Install] Starting installation:", source);

    try {
      setInstalling(true);
      setError(null);

      console.log("[Install] Invoking backend command");

      const result = await invoke("install_shuriken", {
        source,
      });

      console.log("[Install] Backend finished:", result);

      return true;
    } catch (e) {
      console.error("[Install] Installation failed:", e);

      const message = e instanceof Error ? e.message : String(e);

      console.error("[Install] Error message:", message);

      setError(message);
      return false;
    } finally {
      console.log("[Install] Setting installing=false");

      setInstalling(false);
    }
  };

  return {
    install,
    installing,
    progress,
    error,
  };
}