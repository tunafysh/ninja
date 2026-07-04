import { InstallProgress } from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export function useInstallShuriken() {
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenStage: (() => void) | undefined;

    const init = async () => {
      unlistenProgress = await listen<number>("install-progress", (event) => {
        setProgress((prev) => ({
          progress: event.payload,
          stage: prev?.stage ?? "Starting...",
        }));
      });

      unlistenStage = await listen<string>("install-stage", (event) => {
        setProgress((prev) => ({
          progress: prev?.progress ?? 0,
          stage: event.payload,
        }));
      });
    };

    init();

    return () => {
      unlistenProgress?.();
      unlistenStage?.();
    };
  }, []);

  const install = async (source: string) => {
    try {
      setInstalling(true);
      setError(null);
      setProgress({
        progress: 0,
        stage: "Starting...",
      });

      await invoke("install_shuriken", {
        source: source,
      });

      setProgress({
        progress: 100,
        stage: "Finished",
      });
      return true;
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setError(message);
      return false;
    } finally {
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
