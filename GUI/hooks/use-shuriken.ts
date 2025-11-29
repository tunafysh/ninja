import { create } from "zustand"
import { invoke } from "@tauri-apps/api/core"
import { Shuriken } from "@/lib/types"
import { toast } from "sonner"

type ShurikenState = {
  loading: boolean
  error: string | null
  allShurikens: Shuriken[]
  loadingTimeout: NodeJS.Timeout | null

  // actions
  setLoadingDebounced: (value: boolean) => void
  handleError: (err: unknown, context?: string) => void
  refreshShurikens: () => Promise<void>
  updateStatus: (name: string, status: "running" | "stopped") => void
  startShuriken: (name: string) => Promise<void>
  stopShuriken: (name: string) => Promise<void>
  configureShuriken: (name: string) => Promise<void>
  clearError: () => void
}

export const useShuriken = create<ShurikenState>((set, get) => ({
  loading: false,
  error: null,
  allShurikens: [],
  loadingTimeout: null,

  // -------------------------
  // ERROR HANDLER
  // -------------------------
  handleError: (err, context) => {
    const msg = err instanceof Error ? err.message : String(err)
    set({ error: msg })
    toast.error(msg)
    console.error(`[Shuriken ERROR] ${context ?? "unknown"}:`, msg, err)
  },

  // -------------------------
  // LOADING (DEBOUNCED)
  // -------------------------
  setLoadingDebounced: (value: boolean) => {
    const timeout = get().loadingTimeout

    if (timeout) {
      clearTimeout(timeout)
      set({ loadingTimeout: null })
    }

    if (value) {
      const newTimeout = setTimeout(() => set({ loading: true }), 200)
      set({ loadingTimeout: newTimeout })
    } else {
      set({ loading: false })
    }
  },

  // -------------------------
  // REFRESH ALL SHURIKENS
  // -------------------------
  refreshShurikens: async () => {
    const { handleError, setLoadingDebounced } = get()
    setLoadingDebounced(true)

    try {
      const data = await invoke<Shuriken[]>("get_all_shurikens")
      set({ allShurikens: data })
    } catch (err) {
      handleError(err, "refreshShurikens")
      set({ allShurikens: [] })
    } finally {
      setLoadingDebounced(false)
    }
  },

  // -------------------------
  // UPDATE STATUS LOCALLY
  // -------------------------
  updateStatus: (name, status) => {
    set(state => ({
      allShurikens: state.allShurikens.map(s =>
        s.metadata.name === name ? { ...s, status } : s
      ),
    }))
  },

  // -------------------------
  // START SHURIKEN
  // -------------------------
  startShuriken: async (name: string) => {
    const { updateStatus, refreshShurikens, handleError } = get()

    updateStatus(name, "running") // optimistic

    try {
      await invoke("start_shuriken", { name })
    } catch (err) {
      handleError(err, `startShuriken(${name})`)
      updateStatus(name, "stopped")
    }

    refreshShurikens()
  },

  // -------------------------
  // STOP SHURIKEN
  // -------------------------
  stopShuriken: async (name: string) => {
    const { updateStatus, refreshShurikens, handleError } = get()

    updateStatus(name, "stopped") // optimistic

    try {
      await invoke("stop_shuriken", { name })
    } catch (err) {
      handleError(err, `stopShuriken(${name})`)
      updateStatus(name, "running")
    }

    refreshShurikens()
  },

  // -------------------------
  // CONFIGURE SHURIKEN
  // -------------------------
  configureShuriken: async (name: string) => {
    const { handleError, setLoadingDebounced, refreshShurikens } = get()
    
    setLoadingDebounced(true)
    try {
      await invoke("configure_shuriken", { name })
      toast.success(`Configuration applied for ${name}`)
    } catch (err) {
      handleError(err, `configureShuriken(${name})`)
    } finally {
      setLoadingDebounced(false)
      refreshShurikens()
    }
  },

  // -------------------------
  // CLEAR ERROR
  // -------------------------
  clearError: () => set({ error: null }),
}))
