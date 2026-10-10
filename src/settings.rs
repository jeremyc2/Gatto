use std::{collections::BTreeSet, env, fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::global_shortcut::DEFAULT_QUICK_COPY_SHORTCUT;

const APPLICATION_SUPPORT_DIRECTORY: &str = "Gatto";
const LAUNCH_AGENT_NAME: &str = "com.jeremy-chandler.gatto.plist";
const LAUNCH_AGENT_LABEL: &str = "com.jeremy-chandler.gatto";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub organization: Option<String>,
    pub pinned_repositories: BTreeSet<String>,
    pub last_repository: Option<String>,
    pub last_repository_id: Option<u64>,
    #[serde(default = "default_quick_copy_shortcut")]
    pub global_quick_copy_shortcut: String,
    pub global_quick_copy_shortcut_enabled: bool,
    pub close_window_after_copy: bool,
    #[serde(skip)]
    pub start_at_login: bool,
}

fn default_quick_copy_shortcut() -> String {
    DEFAULT_QUICK_COPY_SHORTCUT.to_owned()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            organization: None,
            pinned_repositories: BTreeSet::new(),
            last_repository: None,
            last_repository_id: None,
            global_quick_copy_shortcut: default_quick_copy_shortcut(),
            global_quick_copy_shortcut_enabled: false,
            close_window_after_copy: false,
            start_at_login: false,
        }
    }
}

impl AppSettings {
    /// Returns the repository to restore at launch. A pinned repository is the
    /// stable fallback for preferences saved before an explicit last selection.
    pub fn preferred_repository(&self) -> Option<&str> {
        self.last_repository
            .as_deref()
            .or_else(|| self.pinned_repositories.first().map(String::as_str))
    }

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

    /// Removes every setting this app writes, including its per-user login item.
    pub fn reset() -> Result<()> {
        clear_persistence(&settings_path()?, &launch_agent_path()?)
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
    remove_file_if_present(&path).with_context(|| format!("Could not remove {}", path.display()))
}

fn clear_persistence(settings_path: &PathBuf, launch_agent_path: &PathBuf) -> Result<()> {
    remove_file_if_present(settings_path)
        .with_context(|| format!("Could not remove {}", settings_path.display()))?;
    remove_file_if_present(launch_agent_path)
        .with_context(|| format!("Could not remove {}", launch_agent_path.display()))
}

fn remove_file_if_present(path: &PathBuf) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{AppSettings, clear_persistence};

    #[test]
    fn restores_a_pinned_repository_when_no_last_repository_is_saved() {
        let mut settings = AppSettings::default();
        settings
            .pinned_repositories
            .insert("owner/repository".into());

        assert_eq!(settings.preferred_repository(), Some("owner/repository"));
    }

    #[test]
    fn clearing_persistence_removes_settings_and_login_item() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("gatto-settings-test-{unique_id}"));
        fs::create_dir_all(&directory).expect("test directory can be created");
        let settings_path = directory.join("settings.json");
        let launch_agent_path = directory.join("com.jeremy-chandler.gatto.plist");
        fs::write(&settings_path, "{}").expect("settings file can be created");
        fs::write(&launch_agent_path, "plist").expect("launch agent can be created");

        clear_persistence(&settings_path, &launch_agent_path).expect("persistence can be cleared");

        assert!(!settings_path.exists());
        assert!(!launch_agent_path.exists());
        fs::remove_dir(&directory).expect("test directory can be removed");
    }

    #[test]
    fn older_settings_receive_safe_defaults_for_new_preferences() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
                "organization": "acme",
                "pinned_repositories": [],
                "last_repository": null,
                "last_repository_id": null
            }"#,
        )
        .expect("older settings should remain compatible");

        assert_eq!(settings.global_quick_copy_shortcut, "Command+Shift+U");
        assert!(!settings.global_quick_copy_shortcut_enabled);
        assert!(!settings.close_window_after_copy);
    }
}
