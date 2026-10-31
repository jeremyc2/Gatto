//! Explicit, portable project folders. Media is immutable; the manifest commits
//! a snapshot atomically, so failed writes leave the previous project readable.
#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    fs,
    io::{Read as _, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::model::{AttachmentUploadState, Media, StagedAttachment, VideoUploadLimit};

pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jeremyc2/Gatto/main/schemas/gatto-project.schema.json";
const MANIFEST: &str = "project.json";
const LEGACY_MANIFEST: &str = "gatto-project.json";
const ASSETS: &str = "media";
const LEGACY_ASSETS: &str = "gatto-media";
const SCRATCHPAD_DESCRIPTION: &str = r#""There are only two hard things in Computer Science: cache invalidation and naming things."
- Phil Karlton

We know naming things is hard, so we set this space up for you."#;

#[derive(Deserialize, Serialize)]
struct Manifest {
    #[serde(rename = "$schema")]
    schema: String,
    id: uuid::Uuid,
    title: String,
    version: u32,
    metadata: WorkspaceMetadata,
    attachments: Vec<Attachment>,
}

/// Publish the new name without replacing an existing manifest. Validate the
/// legacy record first and retain it if migration cannot finish.
fn manifest_path(folder: &Path) -> Result<PathBuf> {
    let current = folder.join(MANIFEST);
    let legacy = folder.join(LEGACY_MANIFEST);
    if current.exists() || !legacy.exists() {
        return Ok(current);
    }
    ensure!(
        fs::symlink_metadata(&legacy)?.is_file(),
        "The project manifest must be a regular file"
    );
    let contents = match fs::read(&legacy) {
        Ok(contents) => contents,
        Err(_) if current.exists() => return Ok(current),
        Err(error) => return Err(error.into()),
    };
    let manifest: Manifest = serde_json::from_slice(&contents)?;
    ensure!(manifest.version == 1, "Unsupported project version");
    match fs::hard_link(&legacy, &current) {
        Ok(()) => match fs::remove_file(&legacy) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).context("Could not rename the project manifest to project.json");
        }
    }
    Ok(current)
}

fn media_directory(folder: &Path) -> Result<PathBuf> {
    let current = folder.join(ASSETS);
    let legacy = folder.join(LEGACY_ASSETS);
    if legacy.try_exists()? {
        ensure!(
            fs::symlink_metadata(&legacy)?.is_dir(),
            "Project media must be a folder inside its project"
        );
        ensure!(
            !current.try_exists()? || !legacy.try_exists()?,
            "Both media and gatto-media exist. Resolve the folder conflict before opening this project; neither folder has been changed."
        );
        match fs::rename(&legacy, &current) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && current.is_dir() => {}
            Err(error) => return Err(error).context("Could not rename gatto-media to media"),
        }
    }
    if current.try_exists()? {
        ensure!(
            fs::symlink_metadata(&current)?.is_dir(),
            "Project media must be a folder inside its project"
        );
    }
    Ok(current)
}

#[derive(Deserialize, Serialize)]
struct AnnotationFiles {
    source: String,
    json: String,
}

#[derive(Deserialize, Serialize)]
struct Attachment {
    id: uuid::Uuid,
    file: String,
    sha256: String,
    name: String,
    title: String,
    description: String,
    uploaded_url: Option<String>,
    marker_descriptions: Vec<(u32, String)>,
    is_comparison: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotations: Option<AnnotationFiles>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WorkspaceMetadata {
    pub description: Option<String>,
    pub repository: Option<crate::model::Repository>,
    pub repository_owner: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl WorkspaceMetadata {
    pub fn repository_for_owner(&self, owner: Option<&str>) -> Option<&crate::model::Repository> {
        self.repository.as_ref().filter(|repository| {
            !repository.name.trim().is_empty()
                && self
                    .repository_owner
                    .as_deref()
                    .zip(owner)
                    .is_some_and(|(saved, configured)| saved.eq_ignore_ascii_case(configured))
        })
    }
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone, Debug)]
pub struct ProjectSummary {
    pub id: uuid::Uuid,
    pub title: String,
    pub metadata: WorkspaceMetadata,
}

pub struct ProjectDetails {
    pub summary: ProjectSummary,
    pub storage_size: u64,
}

pub fn summary(folder: &Path) -> Result<ProjectSummary> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path(folder)?)?)?;
    ensure!(manifest.version == 1, "Unsupported project version");
    Ok(ProjectSummary {
        id: manifest.id,
        title: manifest.title,
        metadata: manifest.metadata,
    })
}

pub fn details(folder: &Path) -> Result<ProjectDetails> {
    fn size(path: &Path) -> Result<u64> {
        let meta = fs::symlink_metadata(path)?;
        if meta.is_symlink() {
            return Ok(0);
        }
        if meta.is_file() {
            return Ok(meta.len());
        }
        let mut bytes = 0;
        for child in fs::read_dir(path)? {
            bytes += size(&child?.path())?;
        }
        Ok(bytes)
    }
    Ok(ProjectDetails {
        summary: summary(folder)?,
        storage_size: size(folder)?,
    })
}

pub struct Project {
    pub id: uuid::Uuid,
    pub title: String,
    pub folder: PathBuf,
    pub metadata: WorkspaceMetadata,
    media: HashMap<u64, (Media, String, String)>,
    retired: Vec<String>,
    sidecars: std::collections::HashSet<String>,
}

impl Project {
    /// Release cached media after another transaction has moved it.
    pub fn forget_attachment(&mut self, id: u64) {
        self.media.remove(&id);
    }

    /// Own a new subfolder only after exclusive creation, and save the initial
    /// attachments before exposing it as a project.
    pub fn create_in_parent(
        parent: &Path,
        name: &str,
        attachments: &[StagedAttachment],
    ) -> Result<Self> {
        let parent = parent
            .canonicalize()
            .context("Could not find the parent folder")?;
        let folder = project_folder(&parent, name)?;
        fs::create_dir(&folder).context(
            "Could not create project. If that name already exists, choose another name.",
        )?;
        let result = Self::create_named(folder.clone(), name).and_then(|mut project| {
            project.save(attachments)?;
            Ok(project)
        });
        if let Err(error) = result {
            if let Err(cleanup) = fs::remove_dir_all(&folder) {
                return Err(error.context(format!(
                    "Could not remove the incomplete project folder {}: {cleanup}",
                    folder.display()
                )));
            }
            return Err(error);
        }
        result
    }
    /// Creation requires an empty folder to avoid taking ownership of other data.
    #[cfg(test)]
    pub fn create(folder: PathBuf) -> Result<Self> {
        let name = folder
            .file_name()
            .context("Choose a named project folder")?
            .to_string_lossy()
            .into_owned();
        Self::create_named(folder, &name)
    }

    pub fn create_named(folder: PathBuf, name: &str) -> Result<Self> {
        let name = project_name(name)?;
        fs::create_dir_all(&folder).context("Could not create the project folder")?;
        ensure!(
            fs::read_dir(&folder)?.next().is_none(),
            "The project folder must be empty."
        );
        let mut project = Self {
            id: uuid::Uuid::new_v4(),
            title: name,
            folder: folder.canonicalize()?,
            metadata: WorkspaceMetadata {
                created_at: timestamp(),
                updated_at: timestamp(),
                ..Default::default()
            },
            media: HashMap::new(),
            retired: Vec::new(),
            sidecars: Default::default(),
        };
        project.save(&[])?;
        Ok(project)
    }

    pub fn open(folder: PathBuf) -> Result<(Self, Vec<StagedAttachment>)> {
        let folder = folder
            .canonicalize()
            .context("Could not find the project folder")?;
        let manifest: Manifest = serde_json::from_slice(
            &fs::read(manifest_path(&folder)?)
                .context("This folder does not contain a readable Gatto project")?,
        )
        .context("Could not read the Gatto project manifest")?;
        ensure!(
            manifest.version == 1,
            "This project needs a newer version of Gatto."
        );
        media_directory(&folder)?;
        let metadata = manifest.metadata;
        let mut project = Self {
            id: manifest.id,
            title: manifest.title,
            folder,
            metadata,
            media: HashMap::new(),
            retired: Vec::new(),
            sidecars: Default::default(),
        };
        let mut attachments = Vec::new();
        let mut identities = std::collections::HashSet::new();
        for record in manifest.attachments {
            ensure!(
                identities.insert(record.id),
                "A project contains duplicate attachment IDs"
            );
            // The manifest may be edited outside Gatto. Never follow absolute,
            // parent or symlink paths outside the project's media directory.
            let path = Path::new(&record.file);
            ensure!(
                path.components().count() == 1
                    && matches!(path.components().next(), Some(Component::Normal(_))),
                "Invalid project media path"
            );
            let media_folder = project.folder.join(ASSETS).canonicalize()?;
            ensure!(
                media_folder.parent() == Some(project.folder.as_path()),
                "Project media must stay inside its folder"
            );
            let file = media_folder.join(path).canonicalize()?;
            ensure!(
                file.parent() == Some(media_folder.as_path()),
                "Project media must stay inside its folder"
            );
            let mut item = if file
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(extension, "png" | "jpg" | "jpeg" | "webp" | "gif")
                }) {
                StagedAttachment::from_bytes(record.name.clone(), fs::read(file)?, None)
            } else {
                StagedAttachment::from_path(file, VideoUploadLimit::Paid)
            }
            .with_context(|| format!("Could not open project attachment {}", record.name))?;
            item.persistent_id = record.id;
            item.name = record.name;
            item.title = record.title;
            item.description = record.description;
            if let Some(files) = &record.annotations {
                ensure!(item.image().is_some(), "Only images can have annotations");
                let source = fs::read(project_media_path(&project.folder, &files.source)?)?;
                let json = fs::read(project_media_path(&project.folder, &files.json)?)?;
                let changed = files.source != addressed_name(record.id, &source, "source.png")
                    || files.json != addressed_name(record.id, &json, "annotations.json");
                let annotations = Arc::new(crate::annotation::AnnotationDocument::from_files(
                    source, &json,
                )?);
                if changed {
                    let (pixels, _) = annotations.document().png(None)?;
                    let updated = StagedAttachment::from_bytes(item.name.clone(), pixels, None)?;
                    item.media = updated.media;
                    item.mime_type = updated.mime_type;
                }
                if let Media::Image(image) = &mut item.media {
                    image.annotation_session.restore_annotations(&annotations)?;
                    image.annotation_document = Some(annotations);
                }
                project.sidecars.insert(files.source.clone());
                project.sidecars.insert(files.json.clone());
            }
            let sha256 = media_hash(&item)?;
            if sha256 == record.sha256
                && let Some(url) = record.uploaded_url
            {
                item.upload_state = AttachmentUploadState::Uploaded { url };
            }
            if let Media::Image(image) = &mut item.media {
                image.is_comparison = record.is_comparison;
                image.marker_descriptions = image
                    .annotation_document
                    .as_ref()
                    .map(|annotations| annotations.document().marker_descriptions())
                    .unwrap_or(record.marker_descriptions);
            }
            project
                .media
                .insert(item.id, (item.media.clone(), record.file, sha256));
            attachments.push(item);
        }
        Ok((project, attachments))
    }

    pub fn save(&mut self, attachments: &[StagedAttachment]) -> Result<()> {
        let assets = media_directory(&self.folder)?;
        if !assets.exists() {
            fs::create_dir(&assets)?;
        }
        ensure!(
            assets.canonicalize()? == assets,
            "Project media must stay inside its folder"
        );
        let mut records = Vec::new();
        let mut identities = std::collections::HashSet::new();
        for item in attachments {
            ensure!(
                identities.insert(item.persistent_id),
                "Each attachment needs a unique identity"
            );
            let (file, sha256) = match self.media.get(&item.id) {
                Some((media, file, sha256))
                    if same_media(media, &item.media)
                        && assets.join(file).is_file()
                        && Path::new(file).file_stem().is_some_and(|stem| {
                            stem == format!("{}-{sha256}", item.persistent_id).as_str()
                        }) =>
                {
                    (file.clone(), sha256.clone())
                }
                _ => {
                    let extension = match item.mime_type.as_str() {
                        "image/png" => "png",
                        "image/jpeg" => "jpg",
                        "image/gif" => "gif",
                        "image/webp" => "webp",
                        "video/mp4" => "mp4",
                        "video/quicktime" => "mov",
                        "video/webm" => "webm",
                        _ => anyhow::bail!("Unsupported project media type"),
                    };
                    let sha256 = media_hash(item)?;
                    let name = format!("{}-{sha256}.{extension}", item.persistent_id);
                    let mut file = tempfile::Builder::new()
                        .prefix("attachment-")
                        .suffix(&format!(".{extension}"))
                        .tempfile_in(&assets)?;
                    std::io::copy(&mut item.reader()?, &mut file)?;
                    file.as_file().sync_all()?;
                    file.persist(assets.join(&name))?;
                    if let Some((_, old, _)) = self
                        .media
                        .insert(item.id, (item.media.clone(), name.clone(), sha256.clone()))
                    {
                        self.retired.push(old);
                    }
                    (name, sha256)
                }
            };
            let annotations = if let Some(image) = item.image() {
                let annotations = image
                    .annotation_document
                    .clone()
                    .filter(|doc| !doc.is_empty())
                    .or_else(|| image.annotation_session.saved_annotations());
                if let Some(annotations) = annotations {
                    let source = annotations.source_png()?;
                    let json = annotations.json()?;
                    let source_name = addressed_name(item.persistent_id, &source, "source.png");
                    let json_name = addressed_name(item.persistent_id, &json, "annotations.json");
                    write_media_bytes(&assets, &source_name, &source)?;
                    self.retired.push(source_name.clone());
                    write_media_bytes(&assets, &json_name, &json)?;
                    self.retired.push(json_name.clone());
                    Some(AnnotationFiles {
                        source: source_name,
                        json: json_name,
                    })
                } else {
                    None
                }
            } else {
                None
            };
            records.push(Attachment {
                annotations,
                id: item.persistent_id,
                file,
                sha256,
                name: item.name.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                uploaded_url: item.upload_state.uploaded_url().map(str::to_owned),
                marker_descriptions: item
                    .image()
                    .map(|image| image.marker_descriptions.clone())
                    .unwrap_or_default(),
                is_comparison: item.image().is_some_and(|image| image.is_comparison),
            });
        }
        let sidecars = records
            .iter()
            .filter_map(|record| record.annotations.as_ref())
            .flat_map(|files| [files.source.clone(), files.json.clone()])
            .collect::<std::collections::HashSet<_>>();
        let live_files = records
            .iter()
            .map(|record| record.file.clone())
            .chain(sidecars.iter().cloned())
            .collect::<std::collections::HashSet<_>>();
        self.metadata.updated_at = timestamp().max(self.metadata.updated_at.saturating_add(1));
        let manifest = Manifest {
            metadata: self.metadata.clone(),
            schema: SCHEMA_URL.into(),
            id: self.id,
            title: self.title.clone(),
            version: 1,
            attachments: records,
        };
        write_manifest(&self.folder, &manifest)?;
        self.retired
            .extend(std::mem::replace(&mut self.sidecars, sidecars));
        // Remove only media previously owned by this manifest, after committing it.
        let live: std::collections::HashSet<_> = attachments.iter().map(|item| item.id).collect();
        self.media.retain(|id, (_, file, _)| {
            if live.contains(id) {
                true
            } else {
                self.retired.push(file.clone());
                false
            }
        });
        self.retired.retain(|file| {
            !live_files.contains(file)
                && fs::remove_file(assets.join(file))
                    .is_err_and(|error| error.kind() != std::io::ErrorKind::NotFound)
        });
        Ok(())
    }
}

fn write_manifest(folder: &Path, manifest: &Manifest) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(folder)?;
    serde_json::to_writer_pretty(&mut temporary, manifest)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(folder.join(MANIFEST))
        .context("Could not save the project. Check folder access and free space.")?;
    Ok(())
}

pub fn title(folder: &Path) -> Result<String> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path(folder)?)?)?;
    ensure!(manifest.version == 1, "Unsupported project version");
    Ok(manifest.title)
}

pub fn project_name(name: &str) -> Result<String> {
    let name = name.trim();
    ensure!(
        !name.is_empty() && !name.chars().any(char::is_control),
        "Enter a project name on one line."
    );
    ensure!(
        !name.eq_ignore_ascii_case("Scratchpad"),
        "Scratchpad is reserved for the scratch workspace."
    );
    Ok(name.to_owned())
}

/// Creation uses the display name as one subfolder component. Renames remain
/// metadata-only and do not need to obey filesystem naming constraints.
pub fn project_folder(parent: &Path, name: &str) -> Result<PathBuf> {
    let name = project_name(name)?;
    let path = Path::new(&name);
    ensure!(
        path.components().count() == 1
            && matches!(path.components().next(), Some(Component::Normal(_)))
            && !name.contains('/'),
        "Enter a project name without slashes; '.' and '..' cannot be used."
    );
    Ok(parent.join(name))
}

pub fn distinct_name(
    name: &str,
    folder: &Path,
    projects: &[PathBuf],
    known: &HashMap<PathBuf, String>,
) -> Result<String> {
    let name = project_name(name)?;
    let folder = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    for other in projects {
        if other.canonicalize().ok().as_ref() == Some(&folder) {
            continue;
        }
        let other_name = title(other)
            .or_else(|error| known.get(other).cloned().ok_or(error))
            .with_context(|| {
                format!(
                    "Could not check the project name in {}. Restore access and try again",
                    other.display()
                )
            })?;
        ensure!(
            name.to_lowercase() != other_name.trim().to_lowercase(),
            "That project name is already in use."
        );
    }
    Ok(name)
}

pub fn edit_metadata(
    folder: &Path,
    name: &str,
    description: Option<&str>,
    projects: &[PathBuf],
    known: &HashMap<PathBuf, String>,
) -> Result<String> {
    let name = distinct_name(name, folder, projects, known)?;
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path(folder)?)?)?;
    ensure!(manifest.version == 1, "Unsupported project version");
    manifest.title = name.clone();
    manifest.metadata.description = description.map(str::to_owned);
    manifest.metadata.updated_at = timestamp().max(manifest.metadata.updated_at.saturating_add(1));
    write_manifest(folder, &manifest)?;
    Ok(name)
}

/// Commit the destination first. If removing the source fails, roll the
/// destination back so a failed move keeps the original attachment available.
#[cfg(test)]
pub fn move_attachment(
    source: Option<&Path>,
    destination: &Path,
    attachment: &StagedAttachment,
    remaining: &[StagedAttachment],
) -> Result<String> {
    move_attachments(
        source,
        destination,
        std::slice::from_ref(attachment),
        remaining,
    )
}

pub fn move_attachments(
    source: Option<&Path>,
    destination: &Path,
    attachments: &[StagedAttachment],
    remaining: &[StagedAttachment],
) -> Result<String> {
    let mut source = source
        .map(|folder| Project::open(folder.to_owned()).map(|(project, _)| project))
        .transpose()?;
    let (mut target, mut items) = Project::open(destination.to_owned())?;
    if let Some(source) = &source {
        ensure!(source.id != target.id, "Already in this project.");
    }
    let previous = items.clone();
    for attachment in attachments {
        if let Some(existing) = items
            .iter_mut()
            .find(|item| item.persistent_id == attachment.persistent_id)
        {
            *existing = attachment.clone();
        } else {
            items.push(attachment.clone());
        }
    }
    target.save(&items)?;
    if let Some(source) = &mut source
        && let Err(error) = source.save(remaining)
    {
        if let Err(rollback) = target.save(&previous) {
            anyhow::bail!(
                "Could not remove originals: {error}. Copies remain in the destination: {rollback}"
            );
        }
        return Err(error.context("Move failed; originals remain in their workspace"));
    }
    Ok(target.title)
}

fn addressed_name(id: uuid::Uuid, bytes: &[u8], suffix: &str) -> String {
    format!("{id}-{}.{suffix}", hash_hex(Sha256::digest(bytes).into()))
}

fn project_media_path(folder: &Path, file: &str) -> Result<PathBuf> {
    let path = Path::new(file);
    ensure!(
        path.components().count() == 1
            && matches!(path.components().next(), Some(Component::Normal(_))),
        "Invalid project media path"
    );
    let assets = folder.join(ASSETS).canonicalize()?;
    ensure!(
        assets.parent() == Some(folder),
        "Project media must stay inside its folder"
    );
    let path = assets.join(path).canonicalize()?;
    ensure!(
        path.parent() == Some(assets.as_path()),
        "Project media must stay inside its folder"
    );
    Ok(path)
}

fn write_media_bytes(folder: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    if fs::read(folder.join(name)).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let mut file = tempfile::Builder::new()
        .prefix("annotation-")
        .tempfile_in(folder)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(folder.join(name))?;
    Ok(())
}

pub(crate) fn media_hash(item: &StagedAttachment) -> Result<String> {
    let mut sha256 = Sha256::new();
    let mut reader = item.reader()?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        sha256.update(&buffer[..read]);
    }
    Ok(hash_hex(sha256.finalize().into()))
}

fn hash_hex(hash: [u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(64);
    for byte in hash {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

/// Called only after the user explicitly chooses Delete project in the removal
/// dialog. Remove the project folder without following child symlinks.
pub fn delete_project(folder: &Path) -> Result<()> {
    let folder = folder
        .canonicalize()
        .context("Could not find the project folder")?;
    ensure!(folder.parent().is_some(), "Cannot delete a filesystem root");
    let path = manifest_path(&folder)?.canonicalize()?;
    ensure!(
        path.parent() == Some(folder.as_path()),
        "The project manifest must be inside its folder"
    );
    let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(manifest.version == 1, "Unsupported project version");
    fs::remove_dir_all(folder).context("Could not delete the project folder")?;
    Ok(())
}

/// Resolve a direct project child, rejecting folders and links outside storage.
pub fn project_in_directory(directory: &Path, folder: &Path) -> Result<PathBuf> {
    let directory = directory.canonicalize()?;
    ensure!(
        fs::symlink_metadata(folder)?.is_dir(),
        "Projects must be folders inside the projects directory."
    );
    let folder = folder.canonicalize()?;
    ensure!(
        folder.parent() == Some(directory.as_path()),
        "Projects must be stored inside the projects directory."
    );
    Ok(folder)
}

pub fn project_folders(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if directory.exists() {
        for child in fs::read_dir(directory)? {
            let child = child?;
            if child.file_type()?.is_dir()
                && (child.path().join(MANIFEST).is_file()
                    || child.path().join(LEGACY_MANIFEST).is_file())
            {
                let folder = project_in_directory(directory, &child.path())?;
                paths.push(folder);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn open_scratchpad(folder: PathBuf) -> Result<(Project, Vec<StagedAttachment>)> {
    if manifest_path(&folder)?.exists() {
        let (mut project, attachments) = Project::open(folder)?;
        if project
            .metadata
            .description
            .as_deref()
            .is_none_or(|description| description.trim().is_empty())
        {
            project.metadata.description = Some(SCRATCHPAD_DESCRIPTION.into());
            project.save(&attachments)?;
        }
        return Ok((project, attachments));
    }
    fs::create_dir_all(&folder)?;
    let mut project = Project::create_named(folder, "Scratchpad workspace")?;
    project.title = "Scratchpad".into();
    project.metadata.description = Some(SCRATCHPAD_DESCRIPTION.into());
    project.save(&[])?;
    Ok((project, Vec::new()))
}

/// Move the legacy workspace into place, replacing a conflicting destination.
pub fn migrate_scratchpad(
    data: &Path,
    legacy_repository: Option<crate::model::Repository>,
    owner: Option<String>,
) -> Result<()> {
    let legacy = data.join("scratchpad");
    let projects = data.join("projects");
    let destination = projects.join("Scratchpad");
    fs::create_dir_all(&projects)?;
    if legacy.exists() {
        ensure!(
            fs::symlink_metadata(&legacy)?.is_dir(),
            "Scratchpad must be a folder."
        );
        match fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(&destination),
            Ok(_) => fs::remove_file(&destination),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
        .context("Could not replace Scratchpad")?;
        fs::rename(&legacy, &destination).context("Could not move Scratchpad into projects")?;
    }
    let (mut project, attachments) = open_scratchpad(destination)?;
    if project.metadata.repository.is_none() && legacy_repository.is_some() {
        project.metadata.repository = legacy_repository;
        project.metadata.repository_owner = owner;
        project.save(&attachments)?;
    }
    Ok(())
}

fn same_media(a: &Media, b: &Media) -> bool {
    match (a, b) {
        (Media::Image(a), Media::Image(b)) => Arc::ptr_eq(&a.bytes, &b.bytes),
        (Media::Video(a), Media::Video(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

/// Move onto the target's position, preserving every other item's relative order.
pub fn reorder(attachments: &mut Vec<StagedAttachment>, source: u64, target: u64) -> bool {
    let Some(from) = attachments.iter().position(|item| item.id == source) else {
        return false;
    };
    let Some(to) = attachments.iter().position(|item| item.id == target) else {
        return false;
    };
    if from == to {
        return false;
    }
    let item = attachments.remove(from);
    attachments.insert(to, item);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ImageCopyFormat;

    fn image() -> StagedAttachment {
        StagedAttachment::from_bytes(
            "original.png".into(),
            include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn legacy_manifests_are_discovered_and_migrated_without_changing_project_data() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create_in_parent(directory.path(), "Review", &[]).unwrap();
        let mut item = image();
        item.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/kept".into(),
        };
        project.save(std::slice::from_ref(&item)).unwrap();
        let legacy = project.folder.join(LEGACY_MANIFEST);
        fs::rename(project.folder.join(MANIFEST), &legacy).unwrap();
        fs::rename(
            project.folder.join(ASSETS),
            project.folder.join(LEGACY_ASSETS),
        )
        .unwrap();
        assert_eq!(
            project_folders(directory.path()).unwrap(),
            vec![project.folder.clone()]
        );
        assert_eq!(summary(&project.folder).unwrap().title, "Review");
        assert!(!legacy.exists());
        assert!(project.folder.join("project.json").is_file());
        let (reopened, items) = Project::open(project.folder.clone()).unwrap();
        assert!(project.folder.join("media").is_dir());
        assert!(!project.folder.join(LEGACY_ASSETS).exists());
        assert_eq!(reopened.id, project.id);
        assert_eq!(items[0].persistent_id, item.persistent_id);
        assert_eq!(
            items[0].upload_state.uploaded_url(),
            Some("https://example.com/kept")
        );
    }

    #[test]
    fn conflicting_media_folder_names_preserve_both_folders() {
        let directory = tempfile::tempdir().unwrap();
        let project = Project::create_in_parent(directory.path(), "Review", &[]).unwrap();
        let legacy = project.folder.join(LEGACY_ASSETS);
        fs::create_dir(&legacy).unwrap();
        fs::write(legacy.join("legacy.txt"), b"keep legacy").unwrap();
        let current = project.folder.join(ASSETS);
        fs::write(current.join("current.txt"), b"keep current").unwrap();
        assert!(Project::open(project.folder).is_err());
        assert_eq!(fs::read(legacy.join("legacy.txt")).unwrap(), b"keep legacy");
        assert_eq!(
            fs::read(current.join("current.txt")).unwrap(),
            b"keep current"
        );
    }

    #[test]
    fn existing_project_json_is_authoritative_and_legacy_files_are_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create_in_parent(directory.path(), "Review", &[]).unwrap();
        let legacy = project.folder.join(LEGACY_MANIFEST);
        fs::copy(project.folder.join(MANIFEST), &legacy).unwrap();
        let original = fs::read(&legacy).unwrap();
        project.title = "Current".into();
        project.save(&[]).unwrap();
        assert_eq!(summary(&project.folder).unwrap().title, "Current");
        assert_eq!(fs::read(legacy).unwrap(), original);
    }

    #[test]
    fn unsupported_legacy_manifest_is_left_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let project = Project::create_in_parent(directory.path(), "Review", &[]).unwrap();
        let path = project.folder.join(MANIFEST);
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["version"] = serde_json::json!(2);
        let contents = serde_json::to_vec(&manifest).unwrap();
        fs::write(&path, &contents).unwrap();
        let legacy = project.folder.join(LEGACY_MANIFEST);
        fs::rename(path, &legacy).unwrap();
        assert!(summary(&project.folder).is_err());
        assert!(!project.folder.join(MANIFEST).exists());
        assert_eq!(fs::read(legacy).unwrap(), contents);
    }

    #[test]
    fn discovery_and_resolution_only_accept_direct_project_folders() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("projects");
        assert!(project_folders(&root).unwrap().is_empty());
        fs::create_dir(&root).unwrap();
        let inside = Project::create_in_parent(&root, "Inside", &[]).unwrap();
        let outside = Project::create_in_parent(directory.path(), "Outside", &[]).unwrap();
        let nested = Project::create_in_parent(&inside.folder, "Nested", &[]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside.folder, root.join("Link")).unwrap();
        assert_eq!(project_folders(&root).unwrap(), vec![inside.folder.clone()]);
        assert_eq!(
            project_in_directory(&root, &inside.folder).unwrap(),
            inside.folder
        );
        assert!(project_in_directory(&root, &outside.folder).is_err());
        assert!(project_in_directory(&root, &nested.folder).is_err());
        assert!(project_in_directory(&root, &root.join("Link")).is_err());
    }

    #[test]
    fn opening_requires_the_current_project_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let project = Project::create(directory.path().join("Project")).unwrap();
        let path = project.folder.join(MANIFEST);
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest.as_object_mut().unwrap().remove("metadata");
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(Project::open(project.folder.clone()).is_err());
        assert!(summary(&project.folder).is_err());
    }

    #[test]
    fn new_projects_store_an_unset_description_as_null() {
        let directory = tempfile::tempdir().unwrap();
        let project = Project::create_in_parent(directory.path(), "Project", &[]).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(project.folder.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(manifest["metadata"]["description"], serde_json::Value::Null);
        assert_eq!(project.metadata.description, None);
    }

    #[test]
    fn scratchpad_description_is_populated_and_migrates_existing_blank_value() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("scratchpad");
        let (scratch, _) = open_scratchpad(folder.clone()).unwrap();
        assert_eq!(
            SCRATCHPAD_DESCRIPTION,
            r#""There are only two hard things in Computer Science: cache invalidation and naming things."
- Phil Karlton

We know naming things is hard, so we set this space up for you."#
        );
        assert_eq!(
            scratch.metadata.description.as_deref(),
            Some(SCRATCHPAD_DESCRIPTION)
        );

        let mut scratch = scratch;
        scratch.metadata.description = None;
        scratch.save(&[]).unwrap();
        let (scratch, _) = open_scratchpad(folder).unwrap();
        assert_eq!(
            scratch.metadata.description.as_deref(),
            Some(SCRATCHPAD_DESCRIPTION)
        );
    }

    #[test]
    fn sha256_names_and_streamed_media_hashes_keep_the_existing_format() {
        assert_eq!(
            addressed_name(uuid::Uuid::nil(), b"abc", "source.png"),
            "00000000-0000-0000-0000-000000000000-ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad.source.png"
        );
        // Keep the streaming fixture large independently of artwork compression.
        let mut noise = 0x1234_5678_u32;
        let pixels = image::RgbaImage::from_fn(256, 256, |_, _| {
            noise ^= noise << 13;
            noise ^= noise >> 17;
            noise ^= noise << 5;
            let [red, green, blue, _] = noise.to_le_bytes();
            image::Rgba([red, green, blue, 255])
        });
        let mut output = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(&mut output, image::ImageFormat::Png)
            .unwrap();
        let bytes = output.into_inner();
        assert!(
            bytes.len() > 64 * 1024,
            "Exercise more than one read buffer"
        );
        let item = StagedAttachment::from_bytes("large.png".into(), bytes.clone(), None).unwrap();
        assert_eq!(
            media_hash(&item).unwrap(),
            hash_hex(Sha256::digest(bytes).into())
        );
    }

    #[test]
    fn opening_a_project_rejects_comparison_edits_with_invalid_footer_bounds() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create(directory.path().join("Comparison")).unwrap();
        let mut item = image();
        let mut document =
            crate::annotation::Document::decode(&item.image().unwrap().bytes).unwrap();
        document.comparison = Some(crate::annotation::Comparison::new([
            crate::annotation::Rect {
                x: 16.,
                y: 0.,
                w: 20.,
                h: 30.,
            },
            crate::annotation::Rect {
                x: 52.,
                y: 0.,
                w: 20.,
                h: 30.,
            },
        ]));
        if let Media::Image(image) = &mut item.media {
            image.annotation_document = Some(Arc::new(crate::annotation::AnnotationDocument::new(
                &document,
            )));
        }
        project.save(&[item]).unwrap();
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(project.folder.join(MANIFEST)).unwrap()).unwrap();
        let json_path = project
            .folder
            .join(ASSETS)
            .join(&manifest.attachments[0].annotations.as_ref().unwrap().json);
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&json_path).unwrap()).unwrap();
        for (x, width) in [(-1000., 1.), (f32::MAX, f32::MAX)] {
            for panel in json["comparison"]["panels"].as_array_mut().unwrap() {
                panel["x"] = serde_json::json!(x);
                panel["w"] = serde_json::json!(width);
            }
            fs::write(&json_path, serde_json::to_vec(&json).unwrap()).unwrap();
            assert!(Project::open(project.folder.clone()).is_err());
        }
    }

    #[test]
    fn comparison_flag_survives_project_save_and_open() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create(directory.path().join("Comparison flag")).unwrap();
        let mut comparison = image();
        if let Media::Image(data) = &mut comparison.media {
            data.is_comparison = true;
        }
        let single = image();
        project.save(&[comparison, single]).unwrap();
        let (_, reopened) = Project::open(project.folder).unwrap();
        assert!(reopened[0].image().unwrap().is_comparison);
        assert!(!reopened[1].image().unwrap().is_comparison);
    }

    #[test]
    fn creation_saves_attachments_in_a_new_subfolder_and_preserves_parent_files() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().canonicalize().unwrap();
        fs::write(parent.join("keep.txt"), "keep").unwrap();
        let mut item = image();
        item.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/keep-url".into(),
        };
        let video = StagedAttachment::from_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
            VideoUploadLimit::Standard,
        )
        .unwrap();
        let project =
            Project::create_in_parent(&parent, " Review ", &[item.clone(), video]).unwrap();
        assert_eq!(project.folder, parent.join("Review"));
        assert_eq!(project.title, "Review");
        assert!(!parent.join(MANIFEST).exists());
        assert_eq!(fs::read_to_string(parent.join("keep.txt")).unwrap(), "keep");
        let (_, attachments) = Project::open(project.folder).unwrap();
        assert_eq!(attachments.len(), 2);
        assert_eq!(attachments[0].persistent_id, item.persistent_id);
        assert_eq!(
            attachments[0].upload_state.uploaded_url(),
            Some("https://example.com/keep-url")
        );
        assert!(matches!(attachments[1].media, Media::Video(_)));
    }

    #[test]
    fn creation_rejects_existing_folders_and_names_that_escape_the_parent() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path();
        fs::create_dir(parent.join("Existing")).unwrap();
        assert!(Project::create_in_parent(parent, "Existing", &[]).is_err());
        assert_eq!(fs::read_dir(parent.join("Existing")).unwrap().count(), 0);
        fs::write(parent.join("Existing/keep.txt"), "keep").unwrap();
        assert!(Project::create_in_parent(parent, "Existing", &[]).is_err());
        assert_eq!(
            fs::read_to_string(parent.join("Existing/keep.txt")).unwrap(),
            "keep"
        );
        for invalid in [
            "",
            ".",
            "..",
            "../Escape",
            "/Escape",
            "Nested/Project",
            "Scratchpad",
        ] {
            assert!(
                Project::create_in_parent(parent, invalid, &[]).is_err(),
                "{invalid}"
            );
        }
        assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
    }

    #[test]
    fn failed_initial_save_removes_only_the_new_subfolder() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path();
        fs::write(parent.join("keep.txt"), "keep").unwrap();
        let item = image();
        // Duplicate identities fail after media has started to be written.
        assert!(Project::create_in_parent(parent, "Review", &[item.clone(), item]).is_err());
        assert!(!parent.join("Review").exists());
        assert_eq!(fs::read_to_string(parent.join("keep.txt")).unwrap(), "keep");
    }

    #[test]
    fn scratchpad_metadata_and_completed_urls_survive_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("scratchpad");
        let (mut scratch, _) = open_scratchpad(folder.clone()).unwrap();
        let created = scratch.metadata.created_at;
        let previous = scratch.metadata.updated_at;
        scratch.metadata.description = Some("Notes".into());
        scratch.metadata.repository = Some(crate::model::Repository {
            id: 42,
            name: "images".into(),
        });
        scratch.metadata.repository_owner = Some("acme".into());
        let mut item = image();
        item.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/kept".into(),
        };
        scratch.save(std::slice::from_ref(&item)).unwrap();
        drop(scratch);
        let (scratch, items) = open_scratchpad(folder.clone()).unwrap();
        assert_eq!(scratch.metadata.created_at, created);
        assert!(scratch.metadata.updated_at > previous);
        assert_eq!(scratch.metadata.repository.as_ref().unwrap().id, 42);
        assert_eq!(items[0].persistent_id, item.persistent_id);
        assert_eq!(
            items[0].upload_state.uploaded_url(),
            Some("https://example.com/kept")
        );
        let info = details(&folder).unwrap();
        assert_eq!(info.summary.metadata.description.as_deref(), Some("Notes"));
        let bytes = fs::metadata(folder.join(MANIFEST)).unwrap().len()
            + fs::read_dir(folder.join(ASSETS))
                .unwrap()
                .map(|entry| entry.unwrap().metadata().unwrap().len())
                .sum::<u64>();
        assert_eq!(info.storage_size, bytes);
    }

    #[test]
    fn annotations_remain_editable_after_media_folder_migration_without_undo_history() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create_in_parent(directory.path(), "Annotations", &[]).unwrap();
        let item = image();
        let data = item.image().unwrap();
        let mut retained = data
            .annotation_session
            .open(&data.bytes, data.preview.clone())
            .unwrap();
        retained.document.checkpoint();
        retained.document.marks.push(crate::annotation::Mark::new(
            crate::annotation::Tool::Rectangle,
            crate::annotation::Pos::new(4., 8.),
            Default::default(),
        ));
        let saved = retained.document.edit_state();
        data.annotation_session
            .retain(retained.document, retained.image, saved);
        project.save(&[item]).unwrap();
        fs::rename(
            project.folder.join(ASSETS),
            project.folder.join(LEGACY_ASSETS),
        )
        .unwrap();
        let (_, items) = Project::open(project.folder.clone()).unwrap();
        assert!(!project.folder.join(LEGACY_ASSETS).exists());
        let data = items[0].image().unwrap();
        let reopened = data
            .annotation_session
            .open(&data.bytes, data.preview.clone())
            .unwrap();
        assert_eq!(
            reopened.document.marks.len(),
            1,
            "Saved marks must be editable after reopening the project"
        );
        assert!(!reopened.document.can_undo());
        project.save(&[]).unwrap();
        assert_eq!(
            fs::read_dir(project.folder.join(ASSETS)).unwrap().count(),
            0,
            "Removing the attachment removes its annotation files"
        );
    }

    #[test]
    fn portable_project_round_trip_preserves_order_metadata_media_and_completed_uploads() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Review");
        let mut project = Project::create(folder.clone()).unwrap();
        let mut first = image();
        first.title = "First title".into();
        first.description = "Details \"<&>\"\nsecond line".into();
        first.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/first".into(),
        };
        if let Media::Image(image) = &mut first.media {
            image.marker_descriptions = vec![(1, "Note".into()), (2, String::new())];
        }
        let video = StagedAttachment::from_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
            VideoUploadLimit::Standard,
        )
        .unwrap();
        let original_snippet = first.snippet("https://example.com/first", ImageCopyFormat::Html);
        let persistent_id = first.persistent_id;
        let mut items = vec![first.clone(), video];
        project.save(&items).unwrap();
        let files = fs::read_dir(folder.join(ASSETS)).unwrap().count();
        let target = items[1].id;
        assert!(reorder(&mut items, first.id, target));
        items[0].title = "Video title".into();
        project.save(&items).unwrap();
        assert_eq!(
            fs::read_dir(folder.join(ASSETS)).unwrap().count(),
            files,
            "metadata changes reuse media"
        );
        drop(project);
        let moved = directory.path().join("Moved review");
        fs::rename(folder, &moved).unwrap();
        let (mut project, loaded) = Project::open(moved.clone()).unwrap();
        assert_eq!(loaded[0].title, "Video title");
        assert!(matches!(loaded[0].media, Media::Video(_)));
        assert_eq!(loaded[1].name, "original.png");
        assert_eq!(loaded[1].persistent_id, persistent_id);
        assert_eq!(loaded[1].description, first.description);
        assert_eq!(
            loaded[1].snippet(
                loaded[1].upload_state.uploaded_url().unwrap(),
                ImageCopyFormat::Html
            ),
            original_snippet
        );
        assert_eq!(
            loaded[1].image().unwrap().bytes,
            first.image().unwrap().bytes
        );
        let mut before = Vec::new();
        let mut after = Vec::new();
        items[0].reader().unwrap().read_to_end(&mut before).unwrap();
        loaded[0].reader().unwrap().read_to_end(&mut after).unwrap();
        assert_eq!(before, after);
        project.save(&loaded[1..]).unwrap();
        assert_eq!(fs::read_dir(moved.join(ASSETS)).unwrap().count(), 1);
        project.save(&[]).unwrap();
        assert!(Project::open(moved).unwrap().1.is_empty());
    }

    #[test]
    fn failed_media_save_keeps_the_previous_manifest_and_replaced_media() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create(directory.path().to_owned()).unwrap();
        let original = image();
        project.save(std::slice::from_ref(&original)).unwrap();
        let before = fs::read(directory.path().join(MANIFEST)).unwrap();
        let mut replacement = image();
        replacement.id = original.id;
        replacement.title = "Replacement".into();
        let mut invalid = image();
        invalid.mime_type = "invalid/type".into();
        assert!(project.save(&[replacement.clone(), invalid]).is_err());
        assert_eq!(fs::read(directory.path().join(MANIFEST)).unwrap(), before);
        assert_eq!(
            Project::open(directory.path().to_owned()).unwrap().1[0].title,
            original.title
        );
        project.save(&[replacement]).unwrap();
        assert_eq!(
            fs::read_dir(directory.path().join(ASSETS)).unwrap().count(),
            1
        );
    }

    #[test]
    fn externally_changed_media_drops_only_its_cached_upload_url() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create(directory.path().to_owned()).unwrap();
        let mut items = vec![image(), image()];
        for (index, item) in items.iter_mut().enumerate() {
            item.upload_state = AttachmentUploadState::Uploaded {
                url: format!("https://example.com/{index}"),
            };
        }
        project.save(&items).unwrap();
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(directory.path().join(MANIFEST)).unwrap()).unwrap();
        fs::write(
            directory
                .path()
                .join(ASSETS)
                .join(&manifest.attachments[0].file),
            include_bytes!("../packaging/app-cat-pixel.png"),
        )
        .unwrap();
        let (_, loaded) = Project::open(directory.path().to_owned()).unwrap();
        assert_eq!(loaded[0].upload_state, AttachmentUploadState::Preview);
        assert_eq!(
            loaded[1].upload_state.uploaded_url(),
            Some("https://example.com/1")
        );
    }

    #[test]
    fn creation_and_open_errors_preserve_user_data_and_reject_escaping_paths() {
        let directory = tempfile::tempdir().unwrap();
        let unrelated = directory.path().join("keep.txt");
        fs::write(&unrelated, "keep").unwrap();
        assert!(Project::create(directory.path().to_owned()).is_err());
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep");
        let folder = directory.path().join("Project");
        let mut project = Project::create(folder.clone()).unwrap();
        project.save(&[image()]).unwrap();
        let mut manifest: Manifest =
            serde_json::from_slice(&fs::read(folder.join(MANIFEST)).unwrap()).unwrap();
        for path in ["../keep.txt", "/tmp/outside.png", "nested/image.png"] {
            manifest.attachments[0].file = path.into();
            fs::write(
                folder.join(MANIFEST),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            assert!(Project::open(folder.clone()).is_err());
        }
        manifest.version = 99;
        fs::write(
            folder.join(MANIFEST),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(Project::open(folder).is_err());
    }

    #[test]
    fn reorder_handles_both_directions_and_stale_or_same_ids() {
        let mut items = vec![image(), image(), image()];
        let ids: Vec<_> = items.iter().map(|item| item.id).collect();
        assert!(reorder(&mut items, ids[0], ids[2]));
        assert_eq!(
            items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![ids[1], ids[2], ids[0]]
        );
        assert!(reorder(&mut items, ids[0], ids[1]));
        assert_eq!(items.iter().map(|item| item.id).collect::<Vec<_>>(), ids);
        assert!(!reorder(&mut items, ids[0], ids[0]));
        assert!(!reorder(&mut items, u64::MAX, ids[0]));
        assert!(!reorder(&mut items, ids[0], u64::MAX));
    }

    #[test]
    fn metadata_references_the_published_schema_and_preserves_uuid_identity() {
        let directory = tempfile::tempdir().unwrap();
        let mut project = Project::create(directory.path().join("Review")).unwrap();
        let project_id = project.id;
        let item = image();
        let id = item.persistent_id;
        project.save(&[item]).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(project.folder.join(MANIFEST)).unwrap()).unwrap();
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/gatto-project.schema.json")).unwrap();
        assert_eq!(manifest["$schema"], schema["$id"]);
        assert_eq!(manifest["attachments"][0]["id"], id.to_string());
        assert!(
            manifest["attachments"][0]["file"]
                .as_str()
                .unwrap()
                .starts_with(&id.to_string())
        );
        let (mut reopened, mut items) = Project::open(project.folder.clone()).unwrap();
        assert_eq!(reopened.id, project_id);
        assert_eq!(items[0].persistent_id, id);
        items[0].title = "Edited metadata".into();
        reopened.save(&items).unwrap();
        assert_eq!(
            Project::open(project.folder).unwrap().1[0].persistent_id,
            id
        );
    }

    #[test]
    fn deleting_a_project_removes_its_folder_and_does_not_follow_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Project");
        let mut project = Project::create(folder.clone()).unwrap();
        project.save(&[image()]).unwrap();
        let outside = directory.path().join("Keep");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "keep").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, folder.join("outside-link")).unwrap();
        fs::write(folder.join("notes.txt"), "project contents").unwrap();
        delete_project(&folder).unwrap();
        assert!(!folder.exists());
        assert!(directory.path().is_dir());
        assert_eq!(
            fs::read_to_string(outside.join("keep.txt")).unwrap(),
            "keep"
        );
        assert!(
            delete_project(&outside).is_err(),
            "Only a Gatto project folder may be deleted"
        );
    }

    #[test]
    fn moves_preserve_identity_urls_and_remove_only_the_source_attachment() {
        let directory = tempfile::tempdir().unwrap();
        let mut source =
            Project::create_named(directory.path().join("source-folder"), "Source").unwrap();
        let mut target =
            Project::create_named(directory.path().join("target-folder"), "Target").unwrap();
        let mut item = image();
        item.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/keep-url".into(),
        };
        let keep = image();
        source.save(&[item.clone(), keep.clone()]).unwrap();
        target.save(&[image()]).unwrap();
        let files: Manifest =
            serde_json::from_slice(&fs::read(source.folder.join(MANIFEST)).unwrap()).unwrap();
        move_attachment(
            Some(&source.folder),
            &target.folder,
            &item,
            std::slice::from_ref(&keep),
        )
        .unwrap();
        let (_, remaining) = Project::open(source.folder.clone()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].persistent_id, keep.persistent_id);
        assert!(
            !source
                .folder
                .join(ASSETS)
                .join(&files.attachments[0].file)
                .exists()
        );
        let (_, moved) = Project::open(target.folder.clone()).unwrap();
        assert_eq!(moved.len(), 2);
        assert_eq!(moved[1].persistent_id, item.persistent_id);
        assert_eq!(
            moved[1].upload_state.uploaded_url(),
            Some("https://example.com/keep-url")
        );
        move_attachment(None, &target.folder, &item, &[]).unwrap();
        assert_eq!(Project::open(target.folder).unwrap().1.len(), 2);
    }

    #[test]
    fn failed_move_rolls_back_the_destination_and_preserves_the_original() {
        let directory = tempfile::tempdir().unwrap();
        let mut source = Project::create(directory.path().join("Source")).unwrap();
        let target = Project::create(directory.path().join("Target")).unwrap();
        let item = image();
        source.save(std::slice::from_ref(&item)).unwrap();
        let mut invalid = image();
        invalid.mime_type = "invalid/type".into();
        assert!(move_attachment(Some(&source.folder), &target.folder, &item, &[invalid]).is_err());
        assert_eq!(
            Project::open(source.folder).unwrap().1[0].persistent_id,
            item.persistent_id
        );
        assert!(Project::open(target.folder).unwrap().1.is_empty());
    }

    #[test]
    fn editing_metadata_keeps_the_folder_and_enforces_distinct_names() {
        let directory = tempfile::tempdir().unwrap();
        let mut project =
            Project::create_named(directory.path().join("original-folder"), "Original").unwrap();
        let other =
            Project::create_named(directory.path().join("other-folder"), "Existing name").unwrap();
        let item = image();
        project.save(std::slice::from_ref(&item)).unwrap();
        let folders = vec![project.folder.clone(), other.folder];
        let id = project.id;
        let files = fs::read_dir(project.folder.join(ASSETS))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert!(
            edit_metadata(
                &project.folder,
                " existing NAME ",
                Some("Notes"),
                &folders,
                &HashMap::new()
            )
            .is_err()
        );
        assert!(
            edit_metadata(
                &project.folder,
                " ",
                Some("Notes"),
                &folders,
                &HashMap::new()
            )
            .is_err()
        );
        assert!(
            edit_metadata(
                &project.folder,
                "Scratchpad",
                Some("Notes"),
                &folders,
                &HashMap::new()
            )
            .is_err()
        );
        assert_eq!(
            edit_metadata(
                &project.folder,
                " A new name ",
                Some("First\nSecond"),
                &folders,
                &HashMap::new()
            )
            .unwrap(),
            "A new name"
        );
        let (renamed, items) = Project::open(project.folder.clone()).unwrap();
        assert_eq!(renamed.id, id);
        assert_eq!(renamed.folder, project.folder);
        assert_eq!(
            renamed.metadata.description.as_deref(),
            Some("First\nSecond")
        );
        assert_eq!(renamed.title, "A new name");
        assert_eq!(items[0].persistent_id, item.persistent_id);
        assert_eq!(
            fs::read_dir(project.folder.join(ASSETS))
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>(),
            files
        );
        edit_metadata(
            &project.folder,
            "A new name",
            None,
            &folders,
            &HashMap::new(),
        )
        .unwrap();
        let (cleared, _) = Project::open(project.folder.clone()).unwrap();
        assert_eq!(cleared.metadata.description, None);
    }

    #[test]
    fn scratchpad_migration_preserves_media_repository_and_identity() {
        let directory = tempfile::tempdir().unwrap();
        let legacy = directory.path().join("scratchpad");
        let (mut project, _) = open_scratchpad(legacy.clone()).unwrap();
        let id = project.id;
        let mut item = image();
        item.upload_state = AttachmentUploadState::Uploaded {
            url: "https://example.com/kept".into(),
        };
        project.metadata.repository = Some(crate::model::Repository {
            id: 42,
            name: "project-repo".into(),
        });
        project.metadata.repository_owner = Some("acme".into());
        project.save(std::slice::from_ref(&item)).unwrap();
        migrate_scratchpad(
            directory.path(),
            Some(crate::model::Repository {
                id: 9,
                name: "stale-global".into(),
            }),
            Some("acme".into()),
        )
        .unwrap();
        assert!(!legacy.exists());
        let destination = directory.path().join("projects/Scratchpad");
        let (migrated, items) = Project::open(destination).unwrap();
        assert_eq!(migrated.id, id);
        assert_eq!(migrated.metadata.repository.unwrap().name, "project-repo");
        assert_eq!(items[0].persistent_id, item.persistent_id);
        assert_eq!(
            items[0].upload_state.uploaded_url(),
            Some("https://example.com/kept")
        );
        migrate_scratchpad(directory.path(), None, None).unwrap();
    }

    #[test]
    fn scratchpad_migration_overwrites_destination_and_recovers_repository_selection() {
        let directory = tempfile::tempdir().unwrap();
        let (mut legacy, _) = open_scratchpad(directory.path().join("scratchpad")).unwrap();
        let item = image();
        legacy.save(std::slice::from_ref(&item)).unwrap();
        let projects = directory.path().join("projects");
        std::fs::create_dir_all(&projects).unwrap();
        let collision = projects.join("Scratchpad");
        let mut existing = Project::create_named(collision, "Existing project").unwrap();
        let existing_id = existing.id;
        existing.save(&[image()]).unwrap();
        migrate_scratchpad(
            directory.path(),
            Some(crate::model::Repository {
                id: 42,
                name: "images".into(),
            }),
            Some("acme".into()),
        )
        .unwrap();
        let (scratchpad, items) = Project::open(projects.join("Scratchpad")).unwrap();
        assert_eq!(scratchpad.id, legacy.id);
        assert_ne!(scratchpad.id, existing_id);
        assert_eq!(scratchpad.metadata.repository.unwrap().id, 42);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].persistent_id, item.persistent_id);
        assert_eq!(
            project_folders(&projects).unwrap(),
            vec![projects.join("Scratchpad").canonicalize().unwrap()]
        );
        migrate_scratchpad(directory.path(), None, None).unwrap();
        let (reopened, items) = Project::open(projects.join("Scratchpad")).unwrap();
        assert_eq!(reopened.id, legacy.id);
        assert_eq!(items[0].persistent_id, item.persistent_id);
    }

    #[cfg(unix)]
    #[test]
    fn scratchpad_migration_replaces_a_destination_symlink_without_deleting_its_target() {
        let directory = tempfile::tempdir().unwrap();
        let (legacy, _) = open_scratchpad(directory.path().join("scratchpad")).unwrap();
        let projects = directory.path().join("projects");
        fs::create_dir_all(&projects).unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep.txt"), b"keep").unwrap();
        let destination = projects.join("Scratchpad");
        std::os::unix::fs::symlink(outside.path(), &destination).unwrap();
        migrate_scratchpad(directory.path(), None, None).unwrap();
        assert_eq!(fs::read(outside.path().join("keep.txt")).unwrap(), b"keep");
        assert!(!fs::symlink_metadata(&destination).unwrap().is_symlink());
        assert_eq!(Project::open(destination).unwrap().0.id, legacy.id);
    }
}
