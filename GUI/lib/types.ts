import { LucideIcon } from "lucide-react";

export interface ConfigField  {
    name: string;
    input: "SWITCH" | "NUMBER" | "TEXT";
    script_path?: string;
}

export type Shuriken = {
    shuriken: ShurikenConfig
    config?: Record<string, ConfigParam>
    logs?: LogsConfig
}

export type ShurikenConfig = {
    name: string
    service_name: string
    maintenance: MaintenanceType
    type: string
    status: "running" | "stopped"
    add_path: boolean;
}

// Rust: pub enum MaintenanceType
// Matches serde(tag = "maintenance")
type MaintenanceType =
  | {
      type: "native";
      bin_path: string;
      config_path?: string;
      args?: string[];
    }
  | {
      type: "script";
      script_path: string;
    };


// Rust: pub struct ConfigParam
export interface ConfigParam {
    input: string;
    default?: any; // toml::Value is dynamic
    script: string;
}

// Rust: pub struct LogsConfig
export interface LogsConfig {
    error_log?: string; // PlatformPath as string
}

export type ArmoryItem = {
    name: string,
    label: string,
    synopsis: string,
    description: string,
    version: string,
    authors: string[],
    license: string,
    repository: string,
    dependencies: string[],
    platforms: string[],
    checksum: string
}

export interface LocalArmoryItem extends ArmoryItem {
    installed: boolean,
    localVersion: string
}