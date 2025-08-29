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

  const updateStatus = useCallback((name: string, status: "running" | "stopped") => {
    setAllShurikens(prev =>
      prev.map(s =>
        s.shuriken.name === name
          ? { ...s, status }
          : s
      )
    )
  }, [])

  const startShuriken = useCallback(async (name: string) => {
    updateStatus(name, "running") // optimistic update
    try {
      await invoke('start_shuriken', { name })
      await refreshShurikens()
    } catch (err) {
      handleError(err)
      updateStatus(name, "stopped") // revert if failed
    }
  }, [refreshShurikens, handleError, updateStatus])

  const stopShuriken = useCallback(async (name: string) => {
    updateStatus(name, "stopped") // optimistic update
    try {
      await invoke('stop_shuriken', { name })
      await refreshShurikens()
    } catch (err) {
      handleError(err)
      updateStatus(name, "running") // revert if failed
    }
  }, [refreshShurikens, handleError, updateStatus])

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
