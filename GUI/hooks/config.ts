import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { create } from "zustand";
import type { Config } from "@/lib/types";

interface ConfigStore {
  config: Config | null;
  loading: boolean;
  error: string | null;
  fetchConfig: () => Promise<void>;
  setDevMode: (v: boolean) => Promise<void>;
  addRegistry: (name: string, url: string) => Promise<void>;
  setUpdates: (v: boolean) => void;
  removeRegistry: (name: string) => Promise<void>;
  saveConfig: () => Promise<void>;
}

type RawConfig = {
  checkUpdates?: unknown;
  check_updates?: unknown;
  devMode?: unknown;
  dev_mode?: unknown;
  registries?: unknown;
};

function parseBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function normalizeRegistries(registries: unknown): Map<string, string> {
  if (!registries) {
    return new Map<string, string>();
  }

  if (registries instanceof Map) {
    return new Map(
      Array.from(registries.entries()).filter(
        (entry): entry is [string, string] =>
          typeof entry[0] === "string" && typeof entry[1] === "string",
      ),
    );
  }

  if (typeof registries === "object") {
    return new Map(
      Object.entries(registries).filter(
        (entry): entry is [string, string] =>
          typeof entry[0] === "string" && typeof entry[1] === "string",
      ),
    );
  }

  throw new Error("Invalid registries format from backend");
}

const useConfig = create<ConfigStore>((set) => ({
  config: null,
  loading: false,
  error: null,
  fetchConfig: async () => {
    set({ loading: true, error: null });

    try {
      const raw = await invoke<RawConfig | null>("get_config");

      if (!raw || typeof raw !== "object") {
        throw new Error("Config is null from backend");
      }

      const checkUpdates =
        raw.checkUpdates !== undefined
          ? parseBoolean(raw.checkUpdates, false)
          : parseBoolean(raw.check_updates, false);

      const devMode =
        raw.devMode !== undefined
          ? parseBoolean(raw.devMode, false)
          : parseBoolean(raw.dev_mode, false);

      const normalized: Config = {
        checkUpdates,
        devMode,
        registries: normalizeRegistries(raw.registries),
      };

      set({ config: normalized, loading: false });
    } catch (err) {
      set({ loading: false, error: String(err) });
      toast.error(String(err));
    }
  },
  setDevMode: async (v: boolean) => {
    try {
      await invoke("set_dev_mode", { value: v });
      set((state) => {
        if (!state.config) {
          return {};
        }

        return {
          config: {
            ...state.config,
            devMode: v,
          },
        };
      });
    } catch (err) {
      set({ error: String(err) });
      toast.error(String(err));
    }
  },
  addRegistry: async (name: string, url: string) => {
    try {
      await invoke("add_registry", { name, url });
      set((state) => {
        if (!state.config) {
          return {};
        }

        const newRegistry = new Map(state.config.registries);
        newRegistry.set(name, url);

        return {
          config: {
            ...state.config,
            registries: newRegistry,
          },
        };
      });
    } catch (err) {
      set({ error: String(err) });
      toast.error(String(err));
    }
  },
  setUpdates: async (v: boolean) => {
    try {
      await invoke("set_updates", { value: v });
      set((state) => {
        if (!state.config) {
          return {};
        }

        return {
          config: {
            ...state.config,
            checkUpdates: v,
          },
        };
      });
    } catch (err) {
      set({ error: String(err) });
      toast.error(String(err));
    }
  },
  removeRegistry: async (name: string) => {
    try {
      await invoke("remove_registry", { name });
      set((state) => {
        if (!state.config) {
          return {};
        }

        const newRegistry = new Map(state.config.registries);
        newRegistry.delete(name);

        return {
          config: {
            ...state.config,
            registries: newRegistry,
          },
        };
      });
    } catch (err) {
      set({ error: String(err) });
      toast.error(String(err));
    }
  },
  saveConfig: async () => {
    try {
      await invoke("save_configuration");
    } catch (err) {
      set({ error: String(err) });
      toast.error(String(err));
    }
  },
}));

// Ask Tauri whether config exists. This is async because it talks to the backend.
async function configExists(): Promise<boolean> {
  try {
    const res = await invoke<boolean>("config_exists");
    return !!res;
  } catch (err) {
    console.error("configExists check failed:", err);
    return false;
  }
}

export { configExists, useConfig };
