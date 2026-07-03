import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { Shuriken, ShurikenState } from "@/lib/types";
import { toast } from "sonner";

type UseShuriken = {
  loading: boolean;
  error: string | null;
  allShurikens: Shuriken[];

  // actions
  setLoading: (value: boolean) => void;
  handleError: (err: unknown, context?: string) => void;
  refreshShurikens: () => Promise<void>;
  updateStatus: (name: string, status: ShurikenState) => void;
  startShuriken: (name: string) => Promise<void>;
  stopShuriken: (name: string) => Promise<void>;
  configureShuriken: (name: string) => Promise<void>;
  removeShuriken: (name: string) => Promise<void>;
  clearError: () => void;
};

export const useShuriken = create<UseShuriken>((set, get) => ({
  loading: false,
  error: null,
  allShurikens: [],

  // -------------------------
  // ERROR HANDLER
  // -------------------------
  handleError: (err, context) => {
    const msg = err instanceof Error ? err.message : String(err);
    set({ error: msg });
    toast.error(msg);
    console.error(`[Shuriken ERROR] ${context ?? "unknown"}:`, msg, err);
  },

  // -------------------------
  // LOADING
  // -------------------------
  setLoading: (value: boolean) => {
    set({ loading: value });
  },

  // -------------------------
  // REFRESH ALL SHURIKENS
  // -------------------------
  refreshShurikens: async () => {
    const { handleError, setLoading } = get();

    console.log("[useShuriken] Refreshing shurikens...");
    setLoading(true);

    try {
      await invoke("refresh_shurikens");

      const data = await invoke<Shuriken[]>("get_all_shurikens");

      console.log(`[useShuriken] Retrieved ${data.length} shurikens:`, data);

      set({ allShurikens: data ?? [] });

      console.log("[useShuriken] Store updated:", get().allShurikens);
    } catch (err) {
      handleError(err, "refreshShurikens");
      set({ allShurikens: [] });
    } finally {
      setLoading(false);
    }
  },

  // -------------------------
  // UPDATE STATUS LOCALLY
  // -------------------------
  updateStatus: (name, status) => {
    console.log(`[useShuriken] Updating ${name} status to ${status}`);
    set((state) => ({
      allShurikens: state.allShurikens.map((s) =>
        s.metadata.name === name ? { ...s, state: status } : s,
      ),
    }));
  },

  // -------------------------
  // START SHURIKEN
  // -------------------------
  startShuriken: async (name: string) => {
    console.log(`[useShuriken] Starting shuriken: ${name}`);
    const { updateStatus, handleError } = get();

    updateStatus(name, "Running"); // optimistic

    try {
      await invoke("start_shuriken", { name });
      console.log(`[useShuriken] Successfully started ${name}`);
    } catch (err) {
      handleError(err, `startShuriken(${name})`);
      updateStatus(name, "Idle");
    }
  },

  // -------------------------
  // STOP SHURIKEN
  // -------------------------
  stopShuriken: async (name: string) => {
    console.log(`[useShuriken] Stopping shuriken: ${name}`);
    const { updateStatus, handleError } = get();

    updateStatus(name, "Idle"); // optimistic

    try {
      await invoke("stop_shuriken", { name });
      console.log(`[useShuriken] Successfully stopped ${name}`);
    } catch (err) {
      handleError(err, `stopShuriken(${name})`);
      updateStatus(name, "Running");
    }
  },

  // -------------------------
  // CONFIGURE SHURIKEN
  // -------------------------
  configureShuriken: async (name: string) => {
    console.log(`[useShuriken] Configuring shuriken: ${name}`);
    const { handleError, setLoading, refreshShurikens } = get();

    setLoading(true);
    try {
      await invoke("configure_shuriken", { name });
      console.log(`[useShuriken] Successfully configured ${name}`);
      toast.success(`Configuration applied for ${name}`);
    } catch (err) {
      handleError(err, `configureShuriken(${name})`);
    } finally {
      setLoading(false);
      await refreshShurikens();
    }
  },

  removeShuriken: async (name: string) => {
    console.log(`[useShuriken] Removing shuriken: ${name}`);
    const { handleError, setLoading, refreshShurikens } = get();

    setLoading(true);
    try {
      await invoke("remove_shuriken", { name });
      console.log(`[useShuriken] Successfully removed ${name}`);
      toast.success(`Removed ${name}`);
    } catch (err) {
      handleError(err, `removeShuriken(${name})`);
    } finally {
      setLoading(false);
      await refreshShurikens();
    }
  },
  // -------------------------
  // CLEAR ERROR
  // -------------------------
  clearError: () => {
    console.log("[useShuriken] Clearing error");
    set({ error: null });
  },
}));
