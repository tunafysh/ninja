interface NinjaConfig {
    backups: {
        enabled: boolean,
        path: string,
        schedule: "daily" | "weekly" | "manual"
    },
    checkUpdates: boolean,
    devMode: boolean,
    mcp: {
        enabled: boolean;
        transport: "stdio" | "http";
        hostname: string;
        port: number;
    },
    serverurl: string

    // Add other configuration options as needed
}