use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::global_shortcut::DEFAULT_QUICK_COPY_SHORTCUT;

pub const LOGIN_ARGUMENT: &str = "--login";

const APPLICATION_SUPPORT_DIRECTORY: &str = "Gatto";
const LAUNCH_AGENT_NAME: &str = "com.jeremy-chandler.gatto.plist";
const LAUNCH_AGENT_LABEL: &str = "com.jeremy-chandler.gatto";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    Beta,
    #[default]
    #[serde(other)]
    Stable,
}

impl ReleaseChannel {
    pub fn key(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Beta => "Beta",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub release_channel: ReleaseChannel,
    #[serde(alias = "organization")]
    pub owner: Option<String>,
    pub pinned_repositories: BTreeSet<String>,
    pub last_repository: Option<String>,
    pub last_repository_id: Option<u64>,
    #[serde(default = "default_quick_copy_shortcut")]
    pub global_quick_copy_shortcut: String,
    pub global_quick_copy_shortcut_enabled: bool,
    pub close_window_after_copy: bool,
    pub allow_large_videos: bool,
    pub copy_images_as_markdown: bool,
    #[serde(skip)]
    pub start_at_login: bool,
}

fn default_quick_copy_shortcut() -> String {
    DEFAULT_QUICK_COPY_SHORTCUT.to_owned()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            release_channel: ReleaseChannel::Stable,
            owner: None,
            pinned_repositories: BTreeSet::new(),
            last_repository: None,
            last_repository_id: None,
            global_quick_copy_shortcut: default_quick_copy_shortcut(),
            global_quick_copy_shortcut_enabled: false,
            close_window_after_copy: false,
            allow_large_videos: false,
            copy_images_as_markdown: false,
            start_at_login: false,
        }
    }
}

impl AppSettings {
    pub fn image_copy_format(&self) -> crate::model::ImageCopyFormat {
        if self.copy_images_as_markdown {
            crate::model::ImageCopyFormat::Markdown
        } else {
            crate::model::ImageCopyFormat::Html
        }
    }

    pub fn video_upload_limit(&self) -> crate::model::VideoUploadLimit {
        if self.allow_large_videos {
            crate::model::VideoUploadLimit::Paid
        } else {
            crate::model::VideoUploadLimit::Standard
        }
    }

    /// Repository names and cached IDs belong to the selected owner.
    pub fn set_owner(&mut self, owner: String) -> bool {
        let changed = !self
            .owner
            .as_deref()
            .is_some_and(|saved| saved.eq_ignore_ascii_case(&owner));
        self.owner = Some(owner);
        if changed {
            self.pinned_repositories.clear();
            self.last_repository = None;
            self.last_repository_id = None;
        }
        changed
    }

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

    /// Upgrade an existing login item without enabling login startup for new users.
    pub fn refresh_start_at_login() -> Result<()> {
        let path = launch_agent_path()?;
        if path.exists() {
            refresh_launch_agent(&path, &current_executable()?)?;
        }
        Ok(())
    }

    /// Removes repository preferences and the login item; updater state is separate.
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

    write_launch_agent(&path, &current_executable()?)
}

fn current_executable() -> Result<String> {
    let executable = env::current_exe().context("Could not locate the app executable")?;
    Ok(executable
        .canonicalize()
        .unwrap_or(executable)
        .to_string_lossy()
        .into_owned())
}

fn refresh_launch_agent(path: &Path, executable: &str) -> Result<()> {
    let plist = login_agent_plist(executable);
    let legacy = plist.replace(&format!("<string>{LOGIN_ARGUMENT}</string>\n"), "");
    // Only migrate the exact configuration produced for this executable by older
    // releases. A source build must never repoint an installed app's login item.
    if fs::read_to_string(path).ok().as_deref() == Some(&legacy) {
        write_launch_agent(path, executable)?;
    }
    Ok(())
}

fn login_agent_plist(executable: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{LAUNCH_AGENT_LABEL}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
    <string>{}</string>\n\
    <string>{LOGIN_ARGUMENT}</string>\n\
  </array>\n\
  <key>RunAtLoad</key>\n\
  <true/>\n\
  <key>LimitLoadToSessionType</key>\n\
  <string>Aqua</string>\n\
</dict>\n\
</plist>\n",
        escape_xml(executable)
    )
}

fn write_launch_agent(path: &Path, executable: &str) -> Result<()> {
    let plist = login_agent_plist(executable);
    if fs::read_to_string(path).ok().as_deref() != Some(&plist) {
        let temporary_path = path.with_extension("plist.tmp");
        fs::write(&temporary_path, plist)
            .with_context(|| format!("Could not write {}", temporary_path.display()))?;
        fs::rename(&temporary_path, path)
            .with_context(|| format!("Could not replace {}", path.display()))?;
    }
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

    use super::{AppSettings, clear_persistence, write_settings};

    #[test]
    fn release_channel_defaults_and_persistence_remain_compatible() {
        use super::ReleaseChannel;
        assert_eq!(
            AppSettings::default().release_channel,
            ReleaseChannel::Stable
        );
        for json in ["{}", r#"{"release_channel":"future"}"#] {
            assert_eq!(
                serde_json::from_str::<AppSettings>(json)
                    .unwrap()
                    .release_channel,
                ReleaseChannel::Stable
            );
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        for channel in [ReleaseChannel::Beta, ReleaseChannel::Stable] {
            let settings = AppSettings {
                release_channel: channel,
                ..Default::default()
            };
            write_settings(&settings, &path).unwrap();
            let restored: AppSettings =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(restored.release_channel, channel);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn login_item_passes_background_argument_and_upgrades_existing_configuration() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("gatto-login-test-{unique_id}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("login.plist");
        // Model an enabled login item from a release without the login argument.
        let executable = "/Applications/Gatto & Friends.app/Contents/MacOS/gatto";
        let legacy = super::login_agent_plist(executable)
            .replace(&format!("<string>{}</string>\n", super::LOGIN_ARGUMENT), "");
        fs::write(&path, &legacy).unwrap();
        super::refresh_launch_agent(&path, "/repo/target/debug/gatto").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            legacy,
            "source builds must preserve the installed login path"
        );
        super::refresh_launch_agent(&path, executable).unwrap();
        let output = std::process::Command::new("/usr/bin/plutil")
            .args(["-extract", "ProgramArguments", "json", "-o", "-"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let arguments: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(arguments, [executable, super::LOGIN_ARGUMENT]);
        let expected = fs::read_to_string(&path).unwrap();
        super::write_launch_agent(&path, executable).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn changing_owner_clears_destinations_but_case_changes_preserve_them() {
        let mut settings = AppSettings {
            owner: Some("acme".into()),
            pinned_repositories: ["images".into()].into(),
            last_repository: Some("images".into()),
            last_repository_id: Some(42),
            ..AppSettings::default()
        };
        assert!(!settings.set_owner("ACME".into()));
        assert!(settings.pinned_repositories.contains("images"));
        assert_eq!(settings.last_repository_id, Some(42));
        assert!(settings.set_owner("octocat".into()));
        assert!(settings.pinned_repositories.is_empty());
        assert!(settings.last_repository.is_none());
        assert!(settings.last_repository_id.is_none());
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"owner\":\"octocat\""));
        assert!(!json.contains("organization"));
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.owner.as_deref(), Some("octocat"));
    }

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
                "pinned_repositories": ["images"],
                "last_repository": "images",
                "last_repository_id": 42
            }"#,
        )
        .expect("older settings should remain compatible");

        assert_eq!(settings.owner.as_deref(), Some("acme"));
        assert!(settings.pinned_repositories.contains("images"));
        assert_eq!(settings.last_repository.as_deref(), Some("images"));
        assert_eq!(settings.last_repository_id, Some(42));
        assert_eq!(settings.global_quick_copy_shortcut, "Command+Shift+G");
        assert!(!settings.global_quick_copy_shortcut_enabled);
        assert!(!settings.close_window_after_copy);
        assert!(!settings.allow_large_videos);
        assert!(!settings.copy_images_as_markdown);
        assert_eq!(
            settings.image_copy_format(),
            crate::model::ImageCopyFormat::Html
        );
        assert_eq!(settings.video_upload_limit().bytes(), 10_000_000);
    }

    #[test]
    fn large_video_opt_in_survives_saved_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let mut settings = AppSettings::default();
        assert!(!settings.allow_large_videos);
        settings.allow_large_videos = true;
        write_settings(&settings, &path).unwrap();
        let restored: AppSettings =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(restored.allow_large_videos);
        assert_eq!(restored.video_upload_limit().bytes(), 100_000_000);
        settings.allow_large_videos = false;
        write_settings(&settings, &path).unwrap();
        let restored: AppSettings =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!restored.allow_large_videos);
        assert_eq!(restored.video_upload_limit().bytes(), 10_000_000);
    }

    #[test]
    fn markdown_copy_opt_in_survives_saved_preferences_and_can_be_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert!(!AppSettings::default().copy_images_as_markdown);
        for enabled in [true, false] {
            let settings = AppSettings {
                copy_images_as_markdown: enabled,
                ..Default::default()
            };
            write_settings(&settings, &path).unwrap();
            let restored: AppSettings =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(restored.copy_images_as_markdown, enabled);
            assert_eq!(
                restored.image_copy_format(),
                if enabled {
                    crate::model::ImageCopyFormat::Markdown
                } else {
                    crate::model::ImageCopyFormat::Html
                }
            );
        }
    }
}
