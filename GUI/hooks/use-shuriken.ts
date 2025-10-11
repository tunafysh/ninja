import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Shuriken } from '@/lib/types'
import { toast } from 'sonner'

export const useShuriken = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [allShurikens, setAllShurikens] = useState<Shuriken[]>([])

  const handleError = useCallback((err: unknown, context?: string) => {
    const msg = err instanceof Error ? err.message : String(err)
    setError(msg)
    toast.error(msg)
    console.error(`[Shuriken ERROR] ${context ?? "unknown"}:`, msg, err)
  }, [])

  /** Queries all shurikens directly (no events here) */
  const refreshShurikens = useCallback(async () => {
    console.log("[Shuriken] Refreshing all shurikens...")
    setLoading(true)
    try {
      const data = await invoke<Shuriken[]>('get_all_shurikens')
      console.log("[Shuriken] Retrieved shurikens:", data)
      setAllShurikens(data)
    } catch (err) {
      handleError(err, "refreshShurikens")
      setAllShurikens([])
    } finally {
      setLoading(false)
      console.log("[Shuriken] Finished refresh")
    }
  }, [handleError])

  /** Update local status optimistically */
  const updateStatus = useCallback((name: string, status: "running" | "stopped") => {
  console.log(`[Shuriken] Updating status of "${name}" → ${status}`)
  setAllShurikens(prev =>
    prev.map(s =>
      s.metadata.name === name
        ? { ...s, shuriken: { ...s.metadata, status } } // update inside shuriken
        : s
    )
  )
}, [])

  /** Start shuriken (event listener will confirm success/failure) */
  const startShuriken = useCallback(async (name: string) => {
    console.log(`[Shuriken] Attempting to start "${name}" (optimistic update applied)`)
    updateStatus(name, "running") // optimistic
    try {
      await invoke('start_shuriken', { name })
      console.log(`[Shuriken] Invoked start for "${name}"`)
      refreshShurikens()
    } catch (err) {
      handleError(err, `startShuriken(${name})`)
      updateStatus(name, "stopped") // revert if immediate failure
    }
  }, [handleError, updateStatus])

  /** Stop shuriken (event listener will confirm success/failure) */
  const stopShuriken = useCallback(async (name: string) => {
    console.log(`[Shuriken] Attempting to stop "${name}" (optimistic update applied)`)
    updateStatus(name, "stopped") // optimistic
    try {
      await invoke('stop_shuriken', { name })
      console.log(`[Shuriken] Invoked stop for "${name}"`)
      refreshShurikens()
    } catch (err) {
      handleError(err, `stopShuriken(${name})`)
      updateStatus(name, "running") // revert if immediate failure
    }
  }, [handleError, updateStatus])

  return {
    loading,
    error,
    allShurikens,
    refreshShurikens,
    startShuriken,
    stopShuriken,
    clearError: () => {
      console.log("[Shuriken] Clearing error")
      setError(null)
    },
  }
}