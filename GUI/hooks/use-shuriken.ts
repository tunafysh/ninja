import { useState, useCallback, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Shuriken } from "@/lib/types"
import { toast } from "sonner"

export const useShuriken = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [allShurikens, setAllShurikens] = useState<Shuriken[]>([])
  const loadingTimeout = useRef<NodeJS.Timeout | null>(null)

  const handleError = useCallback((err: unknown, context?: string) => {
    const msg = err instanceof Error ? err.message : String(err)
    setError(msg)
    toast.error(msg)
    console.error(`[Shuriken ERROR] ${context ?? "unknown"}:`, msg, err)
  }, [])

  const setLoadingDebounced = useCallback((value: boolean) => {
    if (loadingTimeout.current) {
      clearTimeout(loadingTimeout.current)
      loadingTimeout.current = null
    }
    if (value) {
      loadingTimeout.current = setTimeout(() => setLoading(true), 200)
    } else {
      setLoading(false)
    }
  }, [])

  const refreshShurikens = useCallback(async () => {
    setLoadingDebounced(true)
    try {
      const data = await invoke<Shuriken[]>("get_all_shurikens")
      setAllShurikens(data)
    } catch (err) {
      handleError(err, "refreshShurikens")
      setAllShurikens([])
    } finally {
      setLoadingDebounced(false)
    }
  }, [handleError, setLoadingDebounced])

  const updateStatus = useCallback((name: string, status: "running" | "stopped") => {
    setAllShurikens(prev =>
      prev.map(s =>
        s.metadata.name === name
          ? { ...s, status }
          : s
      )
    )
  }, [])

  const startShuriken = useCallback(async (name: string) => {
    updateStatus(name, "running")
    try {
      await invoke("start_shuriken", { name })
    } catch (err) {
      handleError(err, `startShuriken(${name})`)
      updateStatus(name, "stopped")
    }
    refreshShurikens()
  }, [handleError, updateStatus, refreshShurikens])

  const stopShuriken = useCallback(async (name: string) => {
    updateStatus(name, "stopped")
    try {
      await invoke("stop_shuriken", { name })
    } catch (err) {
      handleError(err, `stopShuriken(${name})`)
      updateStatus(name, "running")
    }
    refreshShurikens()
  }, [handleError, updateStatus, refreshShurikens])

  const configureShuriken = useCallback(async (name: string) => {
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
  }, [handleError, setLoadingDebounced, refreshShurikens])

  return {
    loading,
    error,
    allShurikens,
    refreshShurikens,
    startShuriken,
    stopShuriken,
    configureShuriken,
    clearError: () => setError(null),
  }
}
