use super::types::Value;
use regex::{Captures, Regex};
use std::{collections::HashMap, error::Error, fmt::Display, path::PathBuf};
use tokio::fs;

#[derive(Debug)]
pub enum TemplateError {
    NotFound(String),
    InvalidConfig(String),
    Internal(String),
    PathNotFound(PathBuf),
}

impl Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::InvalidConfig(msg) => write!(f, "Invalid configuration file: {}", msg),
            TemplateError::Internal(msg) => {
                write!(f, "An unexpected internal error occurred. Message: {}", msg)
            }
            TemplateError::NotFound(tmpl) => write!(f, "Template {} not found!", tmpl),
            TemplateError::PathNotFound(path) => write!(
                f,
                "Path {} not found or it doesn't exist at all",
                path.display()
            ),
        }
    }
}

impl Error for TemplateError {}

pub struct Templater {
    context: HashMap<String, Value>,
}

impl Templater {
    pub fn new(context: HashMap<String, Value>) -> Self {
        Self { context }
    }

    pub fn parse_template(&self, template: &str) -> Result<String, TemplateError> {
        let re = Regex::new(r"\{\{\s*(.*?)\s*\}\}")
            .map_err(|e| TemplateError::Internal(e.to_string()))?;

        Ok(re
            .replace_all(template, |caps: &Captures| {
                let expr = &caps[1];

                // split on `.` for nested lookup
                let mut parts = expr.split('.');
                let first = parts.next();

                if let Some(root_key) = first
                    && let Some(root_val) = self.context.get(root_key)
                {
                    let path = parts.collect::<Vec<_>>().join(".");
                    let val = if path.is_empty() {
                        Some(root_val)
                    } else {
                        root_val.get_path(&path)
                    };
                    return val
                        .map(|v| v.render())
                        .unwrap_or_else(|| format!("{{{{ {} }}}}", expr));
                }

                // If not found, keep the placeholder intact
                format!("{{{{ {} }}}}", expr)
            })
            .to_string())
    }

    pub async fn generate_config(&self, config_path: PathBuf) -> Result<(), TemplateError> {
        let template_path = PathBuf::from(".ninja/config.tmpl");
        let template_path_err = template_path.clone(); //for error purposes
        let template_file = fs::read_to_string(template_path)
            .await
            .map_err(|_| TemplateError::PathNotFound(template_path_err))?;
        let parsed_template = &self.parse_template(template_file.as_str())?;
        let config_path_err = config_path.clone();
        fs::write(config_path, parsed_template)
            .await
            .map_err(|_| TemplateError::PathNotFound(config_path_err))
    }
}
