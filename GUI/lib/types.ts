export type ShurikenState = "Running" | "Idle" | { Error: string };

export type Shuriken = {
  metadata: ShurikenMetadata;
  config?: ShurikenConfig;
  logs?: LogsConfig;
  tools?: Tool[];
  state: ShurikenState;
  dirty: boolean;
};

export type ShurikenConfig = {
  "config-path": String;
  options?: Map<string, Value>;
};

export type Tool = {
  name: string;
  path: string;
};

export type ShurikenMetadata = {
  name: string;
  id: string;
  version: string;
  ports?: number[];
  "check-ports"?: boolean;
  "script-path"?: string;
  type: "daemon" | "executable";
};

export type Value =
  | { type: "String"; value: string }
  | { type: "Number"; value: number }
  | { type: "Bool"; value: boolean }
  | { type: "Map"; value: Record<string, Value> }
  | { type: "Array"; value: Array<Value> };

// Rust: pub struct LogsConfig
export interface LogsConfig {
  "log-path"?: string; // PlatformPath as string
}

export type ArmoryItem =
  | {
      name: string;
      version: string;
      description: string;
      author: string;
      license: string;
      platforms: string[];
      sourceType: "url";
      url: string;
      installed?: boolean;
      label?: string;
      synopsis?: string;
      repository?: string;
      checksum?: string;
      authors?: string[];
    }
  | {
      name: string;
      version: string;
      description: string;
      author: string;
      license: string;
      platforms: string[];
      sourceType: "registry";
      registry: string;
      shuriken: string;
      installed?: boolean;
      label?: string;
      synopsis?: string;
      repository?: string;
      checksum?: string;
      authors?: string[];
    }
  | {
      name: string;
      version: string;
      description: string;
      author: string;
      license: string;
      platforms: string[];
      sourceType: "file" | "path";
      path: string;
      installed?: boolean;
      label?: string;
      synopsis?: string;
      repository?: string;
      checksum?: string;
      authors?: string[];
    };

// Why would i have a fallback type on a discriminated union?

export type LocalArmoryItem = ArmoryItem & {
  installed: boolean;
  localVersion: string;
};

export type Project = {
  name: string;
  readme?: string; // optional snippet or full README content
};

export interface ArmoryMetadata {
  name: string;
  id: string;
  platform: string;
  version: string;
  synopsis?: string | null;
  postinstall?: string | null; // PathBuf → string
  description?: string | null;
  authors?: string[] | null;
  license?: string | null;
}

export type UpdateInfo = {
  version: string;
  date: string | undefined;
  downloadAndInstall: () => void;
  body?: string;
} | null;

export type Config = {
  registries: Map<string, string>;
  devMode: boolean;
  checkUpdates: boolean;
};

export type InstallProgress = {
  progress: number;
  stage: string;
};
