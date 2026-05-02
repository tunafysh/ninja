use dialoguer::{Input, Select, theme::ColorfulTheme};
use ninja::common::types::{ArmoryMetadata, FieldValue};
use owo_colors::OwoColorize;
use std::{collections::HashMap, path::PathBuf};

pub(crate) struct NewShurikenInput {
    pub name: String,
    pub id: String,
    pub version: String,
    pub script_path: PathBuf,
    pub shuriken_type: String,
    pub config_path: Option<PathBuf>,
    pub options: Option<HashMap<String, FieldValue>>,
}

fn prompt_text(
    theme: &ColorfulTheme,
    prompt: &str,
    allow_empty: bool,
) -> Result<String, dialoguer::Error> {
    let input = Input::<String>::with_theme(theme).with_prompt(prompt);
    if allow_empty {
        input.allow_empty(true).interact_text()
    } else {
        input.interact_text()
    }
}

fn prompt_required(theme: &ColorfulTheme, prompt: &str) -> Result<String, dialoguer::Error> {
    prompt_text(theme, prompt, false)
}

fn prompt_optional_string(
    theme: &ColorfulTheme,
    prompt: &str,
) -> Result<Option<String>, dialoguer::Error> {
    let value = prompt_text(theme, prompt, true)?;
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

fn prompt_optional_path(
    theme: &ColorfulTheme,
    prompt: &str,
) -> Result<Option<PathBuf>, dialoguer::Error> {
    Ok(prompt_optional_string(theme, prompt)?.map(PathBuf::from))
}

fn prompt_optional_csv(
    theme: &ColorfulTheme,
    prompt: &str,
) -> Result<Option<Vec<String>>, dialoguer::Error> {
    Ok(prompt_optional_string(theme, prompt)?.and_then(|value| {
        let parts = value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(String::from)
            .collect::<Vec<_>>();

        if parts.is_empty() { None } else { Some(parts) }
    }))
}

fn prompt_confirm(
    theme: &ColorfulTheme,
    prompt: &str,
    default: bool,
) -> Result<bool, dialoguer::Error> {
    dialoguer::Confirm::with_theme(theme)
        .with_prompt(prompt)
        .default(default)
        .interact()
}

fn prompt_config_options(
    theme: &ColorfulTheme,
) -> Result<Option<HashMap<String, FieldValue>>, dialoguer::Error> {
    if !prompt_confirm(theme, "Add configuration options?", false)? {
        return Ok(None);
    }

    let mut map = HashMap::new();
    loop {
        let key = prompt_text(theme, "Enter option key (leave empty to finish)", true)?;
        let key = key.trim().to_string();

        if key.is_empty() {
            break;
        }

        let value = prompt_required(theme, "Enter value for this key")?;
        map.insert(key, FieldValue::from(value));
    }

    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(map))
    }
}

pub(crate) fn collect_new_shuriken_input() -> Result<NewShurikenInput, dialoguer::Error> {
    let theme = ColorfulTheme::default();

    println!("{}", "Manifest section".bold().blue());

    let name = prompt_required(&theme, "Enter the name of the shuriken")?;
    let id = prompt_required(&theme, "Enter the service name")?;
    let version = prompt_text(
        &theme,
        "Enter the version of the shuriken (this is required if you're planning to upload to Armory)",
        true,
    )?;
    let script_path = PathBuf::from(prompt_required(&theme, "Enter the script path")?);

    let shuriken_options = ["daemon", "executable"];
    let choice = Select::with_theme(&theme)
        .with_prompt("Enter the shuriken type")
        .items(shuriken_options)
        .default(0)
        .interact()?;

    println!("{}", "Config section".bold().blue());
    let (config_path, options) = if prompt_confirm(&theme, "Add config?", false)? {
        let conf_path = PathBuf::from(prompt_required(
            &theme,
            "Enter config path for the templater to output (e.g. for Apache 'conf/httpd.conf')",
        )?);
        (Some(conf_path), prompt_config_options(&theme)?)
    } else {
        (None, None)
    };

    Ok(NewShurikenInput {
        name,
        id,
        version,
        script_path,
        shuriken_type: shuriken_options[choice].to_string(),
        config_path,
        options,
    })
}

pub(crate) fn collect_forge_metadata() -> Result<ArmoryMetadata, dialoguer::Error> {
    let theme = ColorfulTheme::default();

    let name = prompt_required(&theme, "Enter the name of the shuriken")?;
    let id = prompt_required(&theme, "Enter the id for this shuriken (Apache -> httpd)")?;
    let platform = prompt_required(
        &theme,
        "Enter the platform this shuriken was designed for \
                 (target triple is preferred but something like \
                 windows-x86_64 is allowed)",
    )?;
    let version = prompt_required(
        &theme,
        "Enter the version for this shuriken \
                 (semver is preferred but anything with numbers will suffice)",
    )?;

    let postinstall = prompt_optional_path(
        &theme,
        "Path for postinstall script (starts from the path you provided as argument, optional)",
    )?;
    let description = prompt_optional_string(
        &theme,
        "Description for the shuriken (will be displayed on the install menu, optional)",
    )?;
    let synopsis = prompt_optional_string(
        &theme,
        "Synopsis (short description) for the shuriken (will be displayed on the install menu, optional)",
    )?;
    let authors = prompt_optional_csv(
        &theme,
        "Authors of this shuriken (optional, comma-separated)",
    )?;
    let repository =
        prompt_optional_string(&theme, "The repository URL for this shuriken (optional)")?;
    let license = prompt_optional_string(
        &theme,
        "The license or licenses the software in this shuriken use \
                 (GPL, MIT or anything similar, optional)",
    )?;

    println!("{}", format!("Generating metadata for '{}'", &name).bold());

    Ok(ArmoryMetadata {
        name,
        id,
        platform,
        version,
        postinstall,
        description,
        authors,
        license,
        synopsis,
        repository,
    })
}
