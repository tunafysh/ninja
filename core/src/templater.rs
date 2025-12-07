use super::types::FieldValue;
use log::{debug, error, info};
use std::{collections::HashMap, env, error::Error, fmt::Display, path::PathBuf};
use tera::{Context, Error as TeraError, ErrorKind, Tera};
use tokio::{fs, sync::RwLock};

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
            TemplateError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            TemplateError::Internal(msg) => write!(f, "Internal template error: {}", msg),
            TemplateError::NotFound(tmpl) => write!(f, "Template '{}' not found", tmpl),
            TemplateError::PathNotFound(path) => {
                write!(f, "Path '{}' not found or inaccessible", path.display())
            }
        }
    }
}

impl Error for TemplateError {}

pub struct Templater {
    context: HashMap<String, FieldValue>,
    root: PathBuf,
    tera: RwLock<Tera>,
}

impl Templater {
    pub fn new(
        mut context: HashMap<String, FieldValue>,
        root_path: PathBuf,
    ) -> Result<Self, TemplateError> {
        debug!("Templater::new: root = {}", root_path.display());

        // Inject default keys
        context
            .entry("platform".to_string())
            .or_insert_with(|| FieldValue::String(env::consts::OS.to_string()));
        context
            .entry("root".to_string())
            .or_insert_with(|| FieldValue::String(root_path.display().to_string()));

        debug!(
            "Templater::new: context size after injection = {}",
            context.len()
        );

        let pattern = root_path.join(".ninja").join("**/*.tmpl");
        let pattern_str = pattern.to_string_lossy();

        info!(
            "Templater::new: compiling Tera templates with pattern '{}'",
            pattern_str
        );

        let tera = Tera::new(&pattern_str).map_err(|e| {
            error!(
                "Templater::new: failed to compile templates (pattern '{}'): {}",
                pattern_str, e
            );
            TemplateError::Internal(format!("Failed to compile templates: {}", e))
        })?;

        info!(
            "Templater::new: initialized successfully (root = {}, pattern = {})",
            root_path.display(),
            pattern_str
        );

        Ok(Self {
            context,
            root: root_path,
            tera: RwLock::new(tera),
        })
    }

    fn to_tera_context(&self) -> Context {
        let mut ctx = Context::new();
        for (key, value) in &self.context {
            ctx.insert(key, value);
        }
        ctx
    }

    async fn render_with_diagnostics(
        &self,
        name: &str,
        template: &str,
    ) -> Result<String, TemplateError> {
        debug!(
            "Templater::render_with_diagnostics: rendering template '{}' (len = {})",
            name,
            template.len()
        );

        let ctx = self.to_tera_context();
        let mut tera_guard = self.tera.write().await;

        match tera_guard.render_str(template, &ctx) {
            Ok(output) => {
                debug!(
                    "Templater::render_with_diagnostics: rendered '{}' (output len = {})",
                    name,
                    output.len()
                );
                Ok(output)
            }
            Err(err) => {
                error!(
                    "Templater::render_with_diagnostics: error rendering '{}': {}",
                    name, err
                );
                Err(Self::diagnose_tera_error(err, name))
            }
        }
    }

    fn diagnose_tera_error(err: TeraError, name: &str) -> TemplateError {
        // Base message
        let mut msg = format!("Error rendering template '{}': {}", name, err);

        match &err.kind {
            ErrorKind::TemplateNotFound(tmpl) => {
                error!(
                    "Templater::diagnose_tera_error: template '{}' not found (referenced as '{}')",
                    tmpl, name
                );
                return TemplateError::NotFound(tmpl.clone());
            }
            other => {
                msg.push_str(&format!(" ({:?})", other));
            }
        }

        // Collect cause chain for context
        let mut source_opt = err.source();
        let mut depth = 0usize;
        while let Some(source) = source_opt {
            depth += 1;
            msg.push_str(&format!(" | cause[{depth}]: {}", source));
            source_opt = source.source();
        }

        error!("Templater::diagnose_tera_error: {}", msg);
        TemplateError::Internal(msg)
    }

    pub async fn parse_template(&self, template: &str) -> Result<String, TemplateError> {
        debug!(
            "Templater::parse_template: inline template (len = {})",
            template.len()
        );
        let result = self.render_with_diagnostics("<inline>", template).await;

        if let Err(err) = &result {
            error!("Templater::parse_template: error: {}", err);
        }

        result
    }

    pub async fn generate_config(&self, config_path: PathBuf) -> Result<(), TemplateError> {
        debug!(
            "Templater::generate_config: target config path = {}",
            config_path.display()
        );

        let template_path = self.root.join(".ninja").join("config.tmpl");
        debug!(
            "Templater::generate_config: template path = {}",
            template_path.display()
        );

        let template_content = fs::read_to_string(&template_path)
            .await
            .map_err(|e| {
                error!(
                    "Templater::generate_config: failed to read template '{}': {}",
                    template_path.display(),
                    e
                );
                TemplateError::PathNotFound(template_path.clone())
            })?;

        debug!(
            "Templater::generate_config: read template (len = {}) from '{}'",
            template_content.len(),
            template_path.display()
        );

        let rendered = self
            .render_with_diagnostics("config.tmpl", &template_content)
            .await?;

        debug!(
            "Templater::generate_config: writing rendered config (len = {}) to '{}'",
            rendered.len(),
            config_path.display()
        );

        fs::write(&config_path, rendered)
            .await
            .map_err(|e| {
                error!(
                    "Templater::generate_config: failed to write config '{}': {}",
                    config_path.display(),
                    e
                );
                TemplateError::PathNotFound(config_path.clone())
            })?;

        info!(
            "Templater::generate_config: config generated at '{}'",
            config_path.display()
        );
        Ok(())
    }
}
