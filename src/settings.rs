use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::global_shortcut::DEFAULT_ADD_FROM_CLIPBOARD_SHORTCUT;

pub const LOGIN_ARGUMENT: &str = "--login";

const APPLICATION_SUPPORT_DIRECTORY: &str = "Gatto";
pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jeremyc2/Gatto/main/schemas/gatto-settings.schema.json";
fn settings_schema() -> String {
    SCHEMA_URL.into()
}
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UploadAction {
    #[default]
    Upload,
    UploadAndCopy,
}

impl UploadAction {
    pub fn label(self, count: usize) -> String {
        let action = match self {
            Self::Upload => "Upload",
            Self::UploadAndCopy => "Upload and copy",
        };
        if count == 0 {
            action.into()
        } else {
            format!(
                "{action} {count} attachment{}",
                if count == 1 { "" } else { "s" }
            )
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    #[serde(rename = "$schema", default = "settings_schema")]
    pub schema: String,
    pub version: u32,
    pub release_channel: ReleaseChannel,
    #[cfg(test)]
    #[serde(skip)]
    pub project_directory_override: Option<PathBuf>,
    pub sidebar_collapsed: bool,
    #[serde(alias = "organization")]
    pub owner: Option<String>,
    pub pinned_repositories: BTreeSet<String>,
    #[serde(alias = "quick_copy_project")]
    pub add_from_clipboard_project: Option<String>,
    pub pinned_projects: BTreeSet<String>,
    #[serde(
        default = "default_add_from_clipboard_shortcut",
        alias = "global_quick_copy_shortcut"
    )]
    pub global_add_from_clipboard_shortcut: String,
    #[serde(alias = "global_quick_copy_shortcut_enabled")]
    pub global_add_from_clipboard_shortcut_enabled: bool,
    pub automatically_copy_add_from_clipboard: bool,
    pub upload_action: UploadAction,
    pub close_window_after_copy: bool,
    pub allow_large_videos: bool,
    pub copy_images_as_markdown: bool,
    pub start_at_login: bool,
}

fn default_add_from_clipboard_shortcut() -> String {
    DEFAULT_ADD_FROM_CLIPBOARD_SHORTCUT.to_owned()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema: settings_schema(),
            version: 1,
            release_channel: ReleaseChannel::Stable,
            #[cfg(test)]
            project_directory_override: None,
            sidebar_collapsed: false,
            owner: None,
            pinned_repositories: BTreeSet::new(),
            add_from_clipboard_project: None,
            pinned_projects: BTreeSet::new(),
            global_add_from_clipboard_shortcut: default_add_from_clipboard_shortcut(),
            global_add_from_clipboard_shortcut_enabled: false,
            automatically_copy_add_from_clipboard: false,
            upload_action: UploadAction::Upload,
            close_window_after_copy: false,
            allow_large_videos: false,
            copy_images_as_markdown: false,
            start_at_login: false,
        }
    }
}

impl AppSettings {
    pub fn project_directory(&self) -> Result<PathBuf> {
        #[cfg(test)]
        if let Some(path) = &self.project_directory_override {
            return Ok(path.clone());
        }
        Ok(data_directory()?.join("projects"))
    }

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
        }
        changed
    }

    pub fn scratchpad_directory(&self) -> Result<PathBuf> {
        Ok(self.project_directory()?.join("Scratchpad"))
    }

    pub fn add_from_clipboard_directory(&self) -> Result<PathBuf> {
        let directory = self.project_directory()?;
        let name = self
            .add_from_clipboard_project
            .as_deref()
            .unwrap_or("Scratchpad");
        let folder = Path::new(name);
        anyhow::ensure!(
            folder.components().count() == 1
                && matches!(
                    folder.components().next(),
                    Some(std::path::Component::Normal(_))
                ),
            "The Add from Clipboard project must be inside the projects folder."
        );
        crate::projects::project_in_directory(&directory, &directory.join(folder))
    }

    pub fn add_from_clipboard_available(&self) -> bool {
        self.add_from_clipboard_project.is_none()
            || self
                .add_from_clipboard_directory()
                .and_then(|folder| crate::projects::summary(&folder))
                .is_ok()
    }

    pub fn load() -> Result<Self> {
        let path = settings_path()?;
        let legacy = home_directory()?
            .join("Library/Application Support")
            .join(APPLICATION_SUPPORT_DIRECTORY)
            .join("settings.json");
        let source = if path.exists() { &path } else { &legacy };
        let mut legacy_selection = None;
        let mut cleanup_legacy = false;
        let mut rewrite_clipboard_names = false;
        let mut settings: Self = if source.exists() {
            let contents = fs::read_to_string(source)
                .with_context(|| format!("Could not read {}", source.display()))?;
            let json: serde_json::Value = serde_json::from_str(&contents)?;
            rewrite_clipboard_names = [
                "quick_copy_project",
                "global_quick_copy_shortcut",
                "global_quick_copy_shortcut_enabled",
            ]
            .iter()
            .any(|key| json.get(*key).is_some());
            cleanup_legacy = json.get("version").is_none()
                || json.get("last_repository").is_some()
                || json.get("last_repository_id").is_some();
            if let Some(name) = json.get("last_repository").and_then(|name| name.as_str()) {
                legacy_selection = Some(crate::model::Repository {
                    name: name.to_owned(),
                    id: json
                        .get("last_repository_id")
                        .and_then(|id| id.as_u64())
                        .unwrap_or(0),
                });
            }
            Self::parse(&contents)
                .with_context(|| format!("Could not parse {}", source.display()))?
        } else {
            Self::default()
        };
        let data = data_directory()?;
        if cleanup_legacy || data.join("scratchpad").exists() {
            crate::projects::migrate_scratchpad(&data, legacy_selection, settings.owner.clone())?;
            settings.save()?;
        }
        if source == &legacy && legacy.exists() {
            settings.start_at_login = launch_agent_path()?.exists();
            if legacy.exists() {
                settings.save()?;
                fs::remove_file(&legacy)?;
            }
        } else if !settings.start_at_login {
            remove_launch_agent()?;
        } else if !launch_agent_path()?.exists() {
            install_launch_agent()?;
        }

        if rewrite_clipboard_names && !cleanup_legacy {
            settings.save()?;
        }
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        write_settings(self, &settings_path()?)
    }

    fn parse(contents: &str) -> Result<Self> {
        let settings: Self = serde_json::from_str(contents)?;
        anyhow::ensure!(settings.version == 1, "Unsupported settings version");
        Ok(settings)
    }

    pub fn set_start_at_login(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            install_launch_agent()?;
        } else {
            remove_launch_agent()?;
        }
        self.start_at_login = enabled;
        self.save()
    }

    /// Upgrade an existing login item without enabling login startup for new users.
    pub fn refresh_start_at_login() -> Result<()> {
        let path = launch_agent_path()?;
        if path.exists() {
            refresh_launch_agent(&path, &current_executable()?)?;
        }
        Ok(())
    }

    /// Removes the complete local data directory and the login item.
    pub fn reset() -> Result<()> {
        clear_persistence(&data_directory()?, &launch_agent_path()?)
    }
}

pub fn data_directory() -> Result<PathBuf> {
    Ok(home_directory()?.join(".gatto"))
}
fn settings_path() -> Result<PathBuf> {
    Ok(data_directory()?.join("settings.json"))
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

fn clear_persistence(data_directory: &PathBuf, launch_agent_path: &PathBuf) -> Result<()> {
    match fs::remove_dir_all(data_directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Could not remove {}", data_directory.display()));
        }
    }
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

    use super::{AppSettings, UploadAction, clear_persistence, write_settings};

    #[test]
    fn workspace_settings_write_schema_and_login_preference() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let settings = AppSettings {
            start_at_login: true,
            ..Default::default()
        };
        write_settings(&settings, &path).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["$schema"], super::SCHEMA_URL);
        assert_eq!(json["start_at_login"], true);
        let restored: AppSettings = serde_json::from_value(json).unwrap();
        assert_eq!(
            restored.project_directory().unwrap(),
            super::data_directory().unwrap().join("projects")
        );
    }

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
            ..AppSettings::default()
        };
        assert!(!settings.set_owner("ACME".into()));
        assert!(settings.pinned_repositories.contains("images"));
        assert!(settings.set_owner("octocat".into()));
        assert!(settings.pinned_repositories.is_empty());
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"owner\":\"octocat\""));
        assert!(!json.contains("organization"));
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.owner.as_deref(), Some("octocat"));
    }

    #[test]
    fn clearing_persistence_removes_entire_data_directory_and_login_item() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("gatto-settings-test-{unique_id}"));
        fs::create_dir_all(&directory).expect("test directory can be created");
        let data_directory = directory.join(".gatto");
        fs::create_dir_all(data_directory.join("projects/Example/media")).unwrap();
        fs::create_dir_all(data_directory.join("scratchpad")).unwrap();
        fs::create_dir_all(data_directory.join("other-state/nested")).unwrap();
        let settings_path = data_directory.join("settings.json");
        let launch_agent_path = directory.join("com.jeremy-chandler.gatto.plist");
        fs::write(&settings_path, "{}").expect("settings file can be created");
        fs::write(&launch_agent_path, "plist").expect("launch agent can be created");
        fs::write(data_directory.join("rooftops.cbor"), b"game data").unwrap();
        fs::write(
            data_directory.join("projects/Example/media/photo.png"),
            b"media",
        )
        .unwrap();
        fs::write(data_directory.join("scratchpad/project.json"), b"{}").unwrap();
        fs::write(
            data_directory.join("other-state/nested/data.bin"),
            b"extra state",
        )
        .unwrap();
        let unrelated = directory.join("unrelated.txt");
        fs::write(&unrelated, b"keep").unwrap();

        clear_persistence(&data_directory, &launch_agent_path).expect("persistence can be cleared");

        assert!(!data_directory.exists());
        assert!(!launch_agent_path.exists());
        assert!(unrelated.exists());
        clear_persistence(&data_directory, &launch_agent_path).expect("repeated reset is harmless");
        fs::remove_file(unrelated).unwrap();
        fs::remove_dir(&directory).expect("test directory can be removed");
    }

    #[test]
    fn older_settings_receive_safe_defaults_for_new_preferences() {
        let settings = AppSettings::parse(
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
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(saved["version"], 1);
        assert!(saved.get("last_repository").is_none());
        assert!(saved.get("last_repository_id").is_none());
        assert_eq!(
            settings.global_add_from_clipboard_shortcut,
            "Command+Shift+G"
        );
        assert!(!settings.global_add_from_clipboard_shortcut_enabled);
        assert!(!settings.close_window_after_copy);
        assert!(!settings.allow_large_videos);
        assert!(!settings.copy_images_as_markdown);
        assert_eq!(settings.upload_action, UploadAction::Upload);
        assert_eq!(
            settings.image_copy_format(),
            crate::model::ImageCopyFormat::Html
        );
        assert_eq!(settings.video_upload_limit().bytes(), 10_000_000);
    }

    #[test]
    fn remembered_upload_action_survives_saving_and_reloading() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        for upload_action in [UploadAction::UploadAndCopy, UploadAction::Upload] {
            let settings = AppSettings {
                upload_action,
                ..Default::default()
            };
            write_settings(&settings, &path).unwrap();
            let loaded = AppSettings::parse(&fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(loaded.upload_action, upload_action);
        }
    }

    #[test]
    fn renamed_clipboard_preferences_preserve_selection_and_shortcut_without_opt_in() {
        let settings = AppSettings::parse(
            r#"{
            "version": 1,
            "quick_copy_project": "Review",
            "global_quick_copy_shortcut": "Command+Shift+U",
            "global_quick_copy_shortcut_enabled": true
        }"#,
        )
        .unwrap();
        assert_eq!(
            settings.add_from_clipboard_project.as_deref(),
            Some("Review")
        );
        assert_eq!(
            settings.global_add_from_clipboard_shortcut,
            "Command+Shift+U"
        );
        assert!(settings.global_add_from_clipboard_shortcut_enabled);
        assert!(!settings.automatically_copy_add_from_clipboard);
        let saved = serde_json::to_value(settings).unwrap();
        assert!(saved.get("quick_copy_project").is_none());
        assert!(saved.get("global_quick_copy_shortcut").is_none());
        assert!(saved.get("global_quick_copy_shortcut_enabled").is_none());
    }

    #[test]
    fn unsupported_settings_versions_are_rejected() {
        assert!(AppSettings::parse(r#"{"version":2}"#).is_err());
        assert!(AppSettings::parse(r#"{"version":0}"#).is_err());
        assert_eq!(AppSettings::parse("{}").unwrap().version, 1);
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
