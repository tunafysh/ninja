import { LucideIcon } from "lucide-react";

export type Shuriken = {
    shuriken: ShurikenMetadata
    config?: ShurikenConfig
    logs?: LogsConfig
    status: "running" | "stopped"
}

export type ShurikenConfig = {
    "config-path": String
    options?: Map<string, Value>
}

export type ShurikenMetadata = {
    name: string
    id: string
    version: string
    maintenance: MaintenanceType
    type: "daemon" | "executable"
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

export type Value =
  | { type: "String"; value: string }
  | { type: "Number"; value: number }
  | { type: "Bool"; value: boolean }
  | { type: "Map"; value: Record<string, Value> }
  | { type: "Array"; value: Array<Value> };

// Rust: pub struct LogsConfig
export interface LogsConfig {
    log_path?: string; // PlatformPath as string
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