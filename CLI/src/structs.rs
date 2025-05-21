pub enum ConfigurationType {
    ScriptConfig,
    ActionConfig,
}


pub struct Service {
    name: String,
    description: String,
    service_name: String,
    version: float,
    bin_path: String,
    config_path: String,
}

pub struct ScriptConfig {
    input: String,
    script: String,
}

pub struct ActionConfig {
    input: String,
    replace: String,
}

pub struct Config {
    configurations: Vec<ConfigurationType>
}

pub struct Logs {
    log_file: String,
}