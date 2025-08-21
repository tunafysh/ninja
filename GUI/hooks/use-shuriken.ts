import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Shuriken } from '@/lib/types'

export const useShuriken = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [allShurikens, setAllShurikens] = useState<Shuriken[]>([])

  const handleError = useCallback((err: unknown) => {
    const msg = err instanceof Error ? err.message : String(err)
    setError(msg)
    console.error('Shuriken Error:', msg)
  }, [])

  const refreshShurikens = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<Shuriken[]>('get_all_shurikens')
      setAllShurikens(data)
    } catch (err) {
      handleError(err)
      setAllShurikens([])
    } finally {
      setLoading(false)
    }
  }, [handleError])

  const updateLocalStatus = useCallback((name: string, status: "running" | "stopped") => {
    setAllShurikens(prev =>
      prev.map(s =>
        s.shuriken.name === name
          ? { ...s, status }
          : s
      )
    )
  }, [])

  const startShuriken = useCallback(async (name: string) => {
    updateLocalStatus(name, "running") // optimistic update
    try {
      await invoke('start_shuriken', { name })
      await refreshShurikens()
    } catch (err) {
      handleError(err)
      updateLocalStatus(name, "stopped") // revert if failed
    }
  }, [refreshShurikens, handleError, updateLocalStatus])

  const stopShuriken = useCallback(async (name: string) => {
    updateLocalStatus(name, "stopped") // optimistic update
    try {
      await invoke('stop_shuriken', { name })
      await refreshShurikens()
    } catch (err) {
      handleError(err)
      updateLocalStatus(name, "running") // revert if failed
    }
  }, [refreshShurikens, handleError, updateLocalStatus])

  return {
    loading,
    error,
    allShurikens,
    refreshShurikens,
    startShuriken,
    stopShuriken,
    clearError: () => setError(null),
  }
}
