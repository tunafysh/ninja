export interface ConfigField  {
    name: string;
    input: "SWITCH" | "YESNO" | "NUMBER" | "TEXT";
    replace: string;
}