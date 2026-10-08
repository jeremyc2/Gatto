use std::{collections::BTreeSet, env, fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

const APPLICATION_SUPPORT_DIRECTORY: &str = "Gatto";
const LAUNCH_AGENT_NAME: &str = "com.jeremy-chandler.gatto.plist";
const LAUNCH_AGENT_LABEL: &str = "com.jeremy-chandler.gatto";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub organization: Option<String>,
    pub pinned_repositories: BTreeSet<String>,
    #[serde(skip)]
    pub start_at_login: bool,
}

impl AppSettings {
    pub fn load() -> Result<Self> {
        let path = settings_path()?;
        let mut settings = if path.exists() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Could not read {}", path.display()))?;
            serde_json::from_str(&contents)
                .with_context(|| format!("Could not parse {}", path.display()))?
        } else {
            Self::default()
        };
        settings.start_at_login = launch_agent_path()?.exists();
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        write_settings(self, &settings_path()?)
    }

    pub fn set_start_at_login(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            install_launch_agent()?;
        } else {
            remove_launch_agent()?;
        }
        self.start_at_login = enabled;
        Ok(())
    }
}

fn settings_path() -> Result<PathBuf> {
    Ok(home_directory()?
        .join("Library/Application Support")
        .join(APPLICATION_SUPPORT_DIRECTORY)
        .join("settings.json"))
}

fn launch_agent_path() -> Result<PathBuf> {
    Ok(home_directory()?
        .join("Library/LaunchAgents")
        .join(LAUNCH_AGENT_NAME))
}

fn write_settings(settings: &AppSettings, path: &PathBuf) -> Result<()> {
    let directory = path
        .parent()
        .context("The settings file has no parent directory")?;
    fs::create_dir_all(directory)
        .with_context(|| format!("Could not create {}", directory.display()))?;

    let temporary_path = path.with_extension("json.tmp");
    let contents = serde_json::to_string_pretty(settings)?;
    fs::write(&temporary_path, format!("{contents}\n"))
        .with_context(|| format!("Could not write {}", temporary_path.display()))?;
    fs::rename(&temporary_path, path)
        .with_context(|| format!("Could not replace {}", path.display()))?;
    Ok(())
}

fn home_directory() -> Result<PathBuf> {
    let home = env::var_os("HOME").context("The HOME environment variable is not set")?;
    let path = PathBuf::from(home);
    if !path.is_absolute() {
        bail!("The HOME environment variable is not an absolute path");
    }
    Ok(path)
}

fn install_launch_agent() -> Result<()> {
    let path = launch_agent_path()?;
    let directory = path
        .parent()
        .context("The LaunchAgents path has no parent directory")?;
    fs::create_dir_all(directory)
        .with_context(|| format!("Could not create {}", directory.display()))?;

    let executable = env::current_exe().context("Could not locate the app executable")?;
    let executable = executable
        .canonicalize()
        .unwrap_or(executable)
        .to_string_lossy()
        .into_owned();
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{LAUNCH_AGENT_LABEL}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
    <string>{}</string>\n\
  </array>\n\
  <key>RunAtLoad</key>\n\
  <true/>\n\
  <key>LimitLoadToSessionType</key>\n\
  <string>Aqua</string>\n\
</dict>\n\
</plist>\n",
        escape_xml(&executable)
    );
    fs::write(&path, plist).with_context(|| format!("Could not write {}", path.display()))?;
    Ok(())
}

fn remove_launch_agent() -> Result<()> {
    let path = launch_agent_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Could not remove {}", path.display())),
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
