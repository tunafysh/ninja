export interface ConfigField  {
    name: string;
    input: "SWITCH" | "YESNO" | "NUMBER" | "TEXT";
    replace: string;
}

export interface Shuriken { 
    name: string;
    description: string;
    icon: string;
    config_path: string;
    log_path: string;
    
    config: ConfigField[];
}