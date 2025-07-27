import { LucideIcon } from "lucide-react";

export interface ConfigField  {
    name: string;
    input: "SWITCH" | "YESNO" | "NUMBER" | "TEXT";
    replace: string;
}



// Rust: pub struct ShurikenConfig
export type Shuriken = {
    name: string
    service_name: string
    maintenance: MaintenanceType
    type: ShurikenType
    config?: Record<string, ConfigParam>
    status: "running" | "stopped"
    icon: LucideIcon
    color: string
    logs?: LogsConfig
}

// Rust: pub enum MaintenanceType
export type MaintenanceType =
    ({ kind: 'Native'} & {bin_path: string; config_path?: string; args?: string[] }) |
    ({ kind: 'Script'} & {script_path: string });

// Rust: pub enum ShurikenType
export type ShurikenType = { kind: 'Daemon'; ports?: number[]; health_check?: string } | { kind: 'Executable'; add_path: boolean };

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