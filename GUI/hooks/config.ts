import { Config } from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { toast } from "sonner";

interface ConfigStore {
    config: Config | null
    loading: boolean
    error: string | null
    fetchConfig: () => Promise<void>
    toggleDevMode: () => Promise<void>
    addRegistry: (name: string, url: string) => Promise<void>
    toggleUpdates: () => void
    removeRegistry: (name: string) => Promise<void>
}

const useConfig = create<ConfigStore>((set) => ({
    config: null,
    loading: false,
    error: null,
    fetchConfig: async () => {
        set({ loading: true, error: null})

        try{
            const config = await invoke<Config>("get_config")
            set({config, loading: false})
        }
        catch (err) {
            set({loading: false, error: String(err)})
            toast.error(String(err))
        }
    },
    toggleDevMode: async () => {
        try {
            await invoke("toggle_dev_mode")
            set((state) => {
                if (!state.config) {
                    return {}
                }

                return {
                    config: {
                        ...state.config,
                        devMode: !state.config.devMode,
                    },
                }
            })
        } catch (err) {
            set({ error: String(err) })
            toast.error(String(err))
        }
    },
    addRegistry: async (name: string, url: string) => {
        try {
            await invoke("add_registry", { name, url })
            set((state) => {
                if (!state.config) {
                    return {}
                }

                const newRegistry = state.config.registries

                newRegistry.set(name, url)

                return {
                    config: {
                        ...state.config,
                        registries: newRegistry
                    }
                }
            })
        } catch (err) {
            set({ error: String(err) })
            toast.error(String(err))
        }
    },
    toggleUpdates: async () => {
        try {
            await invoke("toggle_updates")
            set((state) => {
                if (!state.config) {
                    return {}
                }

                return {
                    config: {
                        ...state.config,
                        checkUpdates: !state.config.checkUpdates
                    }
                }
            })
        } catch (err) {
            set({ error: String(err) })
            toast.error(String(err))
        }
    },
    removeRegistry: async (name: string) => {
        try {
            await invoke("remove_registry", { name })
            set((state) => {
                if (!state.config) {
                    return {}
                }

                const newRegistry = new Map(state.config.registries)
                newRegistry.delete(name)

                return {
                    config: {
                        ...state.config,
                        registries: newRegistry
                    }
                }
            })
        } catch (err) {
            set({ error: String(err) })
            toast.error(String(err))
        }
    }

}))

function configExists(): boolean {
    return false
}

export { useConfig, configExists }