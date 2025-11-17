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
        debug!("Initializing Templater at root: {}", root_path.display());

        // Inject default keys
        context
            .entry("platform".to_string())
            .or_insert_with(|| FieldValue::String(env::consts::OS.to_string()));
        context
            .entry("root".to_string())
            .or_insert_with(|| FieldValue::String(root_path.display().to_string()));

        let pattern = root_path.join(".ninja").join("**/*.tmpl");
        let pattern_str = pattern.to_string_lossy();

        let tera = Tera::new(&pattern_str)
            .map_err(|e| TemplateError::Internal(format!("Failed to compile templates: {}", e)))?;

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
        let ctx = self.to_tera_context();
        self.tera
            .write()
            .await
            .render_str(template, &ctx)
            .map_err(|err| Self::diagnose_tera_error(err, name))
    }

    fn diagnose_tera_error(err: TeraError, name: &str) -> TemplateError {
        // Base message: include template name and the Tera-formatted message
        let mut msg = format!("Error rendering template '{}': {}", name, err);

        // Special-case known error kinds
        match &err.kind {
            ErrorKind::TemplateNotFound(tmpl) => {
                // Template not found: return NotFound immediately
                return TemplateError::NotFound(tmpl.clone());
            }
            // We keep other kinds for context; append the kind's debug form
            other => {
                msg.push_str(&format!(" ({:?})", other));
            }
        }

        // Walk the source chain (if any) to give more context about underlying causes
        let mut source_opt = err.source();
        while let Some(source) = source_opt {
            // `source` implements Display, so include its message
            msg.push_str(&format!(" | cause: {}", source));
            source_opt = source.source();
        }

        // Log the diagnostic for easier debugging in logs
        error!("{}", msg);

        TemplateError::Internal(msg)
    }

    pub async fn parse_template(&self, template: &str) -> Result<String, TemplateError> {
        debug!("Rendering inline template...");
        self.render_with_diagnostics("<inline>", template).await
    }

    pub async fn generate_config(&self, config_path: PathBuf) -> Result<(), TemplateError> {
        let template_path = self.root.join(".ninja").join("config.tmpl");
        debug!("Reading template: {}", template_path.display());

        let template_content = fs::read_to_string(&template_path)
            .await
            .map_err(|_| TemplateError::PathNotFound(template_path.clone()))?;

        let rendered = self
            .render_with_diagnostics("config.tmpl", &template_content)
            .await?;
        debug!("Writing rendered template to {}", config_path.display());

        fs::write(&config_path, rendered)
            .await
            .map_err(|_| TemplateError::PathNotFound(config_path.clone()))?;

        info!("Config generated successfully at {}", config_path.display());
        Ok(())
    }
}
