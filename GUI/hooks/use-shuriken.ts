// use-shuriken.ts
import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { toast } from 'sonner'
import { ShurikenConfig, ShurikenMetadata, LogsConfig } from '@/lib/types'

export type ShurikenRuntimeState = 'Stopped' | 'Starting' | 'Running' | 'Stopping'

export interface Shuriken {
  metadata: ShurikenMetadata
  config?: ShurikenConfig
  logs?: LogsConfig
  runtime: ShurikenRuntimeState
}

export const useShuriken = () => {
  const [allShurikens, setAllShurikens] = useState<Shuriken[]>([])
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
      const data = await invoke<Shuriken[]>('refresh_shurikens')
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
      const updated = await invoke<Shuriken>('start_shuriken', { name })
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
      const updated = await invoke<Shuriken>('stop_shuriken', { name })
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
        const updated = await invoke<Shuriken>('configure_shuriken', {
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
