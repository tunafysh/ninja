// Type definitions for Tauri API
export interface TauriAPI {
    core: {
      invoke: <T>(cmd: string, args?: any) => Promise<T>
    }
  }
  
  // Helper function to check if Tauri is available
  export function isTauriAvailable(): boolean {
    return typeof window !== "undefined" && window.__TAURI__ !== undefined && window.__TAURI__.core !== undefined
  }
  
  // Helper function to execute a command via Tauri
  export async function executeCommand(command: string): Promise<string> {
    if (!isTauriAvailable()) {
      throw new Error("Tauri is not available")
    }
  
    return window.__TAURI__.core.invoke("execute_command", { command })
  }
  
  // Helper function to get the current directory
  export async function getCurrentDirectory(): Promise<string> {
    if (!isTauriAvailable()) {
      throw new Error("Tauri is not available")
    }
  
    return window.__TAURI__.core.invoke("get_current_dir")
  }
  
  // Declare global window interface
  declare global {
    interface Window {
      __TAURI__?: TauriAPI
    }
  }
  
  