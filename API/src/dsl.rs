use regex::Regex;
use toml::Value;

#[derive(Debug, Clone)]
pub enum Command {
    Set { key: String, value: Value },
    Toggle { key: String },
    Show { key: Option<String> },
    List,
    Get { key: String },
}

pub fn parse_command(input: &str) -> Option<Command> {
    let input = input.trim();

    // SET command: set key = value
    let re_set = Regex::new(r"^set\s+(\w+)\s*=\s*(.+)$").unwrap();
    if let Some(caps) = re_set.captures(input) {
        let key = caps[1].to_string();
        let value_str = &caps[2];
        // Try to parse as integer/boolean, fallback to string
        let value = if let Ok(num) = value_str.parse::<i64>() {
            Value::Integer(num)
        } else if let Ok(b) = value_str.parse::<bool>() {
            Value::Boolean(b)
        } else {
            Value::String(value_str.to_string())
        };
        return Some(Command::Set { key, value });
    }

    // TOGGLE command: toggle key
    let re_toggle = Regex::new(r"^toggle\s+(\w+)$").unwrap();
    if let Some(caps) = re_toggle.captures(input) {
        return Some(Command::Toggle { key: caps[1].to_string() });
    }

    // SHOW command: show [key]
    let re_show = Regex::new(r"^show(?:\s+(\w+))?$").unwrap();
    if let Some(caps) = re_show.captures(input) {
        let key = caps.get(1).map(|m| m.as_str().to_string());
        return Some(Command::Show { key });
    }

    // GET command: get key
    let re_get = Regex::new(r"^get\s+(\w+)$").unwrap();
    if let Some(caps) = re_get.captures(input) {
        return Some(Command::Get { key: caps[1].to_string() });
    }

    // LIST command: list
    if input == "list" {
        return Some(Command::List);
    }

    None // Unknown command
}