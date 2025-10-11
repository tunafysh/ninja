// use-shuriken.ts
import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { toast } from 'sonner'

export type ShurikenRuntimeState = 'Stopped' | 'Starting' | 'Running' | 'Stopping'

export interface ShurikenFull {
  metadata: {
    name: string
    [key: string]: any
  }
  config?: Record<string, any>
  logs?: Record<string, any>
  runtime: ShurikenRuntimeState
}

export const useShuriken = () => {
  const [allShurikens, setAllShurikens] = useState<ShurikenFull[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleError = useCallback((err: unknown, context?: string) => {
    const msg = err instanceof Error ? err.message : String(err)
    setError(msg)
    toast.error(msg)
    console.error(`[Shuriken ERROR] ${context ?? 'unknown'}:`, msg, err)
  }, [])

  /** Fetch the current full list of shurikens */
  const refreshShurikens = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<ShurikenFull[]>('refresh_shurikens')
      setAllShurikens(data)
    } catch (err) {
      handleError(err, 'refreshShurikens')
      setAllShurikens([])
    } finally {
      setLoading(false)
    }
  }, [handleError])

  /** Start a shuriken */
  const startShuriken = useCallback(async (name: string) => {
    try {
      const updated = await invoke<ShurikenFull>('start_shuriken', { name })
      setAllShurikens(prev =>
        prev.map(s => (s.metadata.name === name ? updated : s))
      )
    } catch (err) {
      handleError(err, `startShuriken(${name})`)
    }
  }, [handleError])

  /** Stop a shuriken */
  const stopShuriken = useCallback(async (name: string) => {
    try {
      const updated = await invoke<ShurikenFull>('stop_shuriken', { name })
      setAllShurikens(prev =>
        prev.map(s => (s.metadata.name === name ? updated : s))
      )
    } catch (err) {
      handleError(err, `stopShuriken(${name})`)
    }
  }, [handleError])

  /** Configure a shuriken */
  const configureShuriken = useCallback(
    async (name: string, fields: Record<string, any>) => {
      try {
        const updated = await invoke<ShurikenFull>('configure_shuriken', {
          name,
          fields,
        })
        setAllShurikens(prev =>
          prev.map(s => (s.metadata.name === name ? updated : s))
        )
      } catch (err) {
        handleError(err, `configureShuriken(${name})`)
      }
    },
    [handleError]
  )

  /** Clear the last error */
  const clearError = useCallback(() => setError(null), [])

  return {
    allShurikens,
    loading,
    error,
    refreshShurikens,
    startShuriken,
    stopShuriken,
    configureShuriken,
    clearError,
  }
}
