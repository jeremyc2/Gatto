//! Best-effort cleanup of the installer's mounted disk, without reading its
//! files or touching the downloaded DMG. Packaging records the volume UUID in
//! the app before signing; system metadata supplies the other side of the pair.

use std::{
    env,
    io::Read as _,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{
    NSBundle, NSData, NSJSONSerialization, NSJSONWritingOptions, NSPropertyListMutabilityOptions,
    NSPropertyListSerialization, NSString, NSURL,
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::diagnostics::Diagnostics;

const INSTALLER_UUID_KEY: &str = "GattoInstallerVolumeUUID";
const METADATA_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_METADATA_BYTES: u64 = 1024 * 1024;
const MAX_VOLUMES: usize = 32;

unsafe extern "C" {
    fn getuid() -> u32;
}

/// Called after the main window and menu bar have been installed. All discovery
/// and ejection work stays off the UI thread and never presents cleanup UI.
pub fn start(diagnostics: Diagnostics) {
    let worker_diagnostics = diagnostics.clone();
    if thread::Builder::new()
        .name("installer-cleanup".into())
        .spawn(move || {
            autoreleasepool(|_| {
                let Some(uuid) = installed_bundle_uuid() else {
                    return;
                };
                // SAFETY: getuid has no arguments or preconditions.
                let uid = unsafe { getuid() };
                match cleanup(&uuid, uid) {
                    Some(true) => worker_diagnostics.info("Installer disk ejected."),
                    Some(false) => worker_diagnostics.warn("Installer disk could not be ejected."),
                    None => {}
                }
            });
        })
        .is_err()
    {
        diagnostics.warn("Installer cleanup could not start.");
    }
}

fn installed_bundle_uuid() -> Option<String> {
    let executable = env::current_exe().ok()?.canonicalize().ok()?;
    let home = PathBuf::from(env::var_os("HOME")?).canonicalize().ok()?;
    let bundle = NSBundle::mainBundle();
    let path = PathBuf::from(bundle.bundlePath().to_string());
    // Check the actual executable as well as the bundle. In particular, do not
    // clean up from a development build, mounted image, or translocated app.
    if !is_installed_executable(&executable, &home)
        || path.canonicalize().ok()?.join("Contents/MacOS/gatto") != executable
        || bundle.bundleIdentifier()?.to_string() != "com.jeremy-chandler.gatto"
    {
        return None;
    }
    let value = bundle.objectForInfoDictionaryKey(&NSString::from_str(INSTALLER_UUID_KEY))?;
    normalized_uuid(&value.downcast::<NSString>().ok()?.to_string())
}

fn is_installed_executable(executable: &Path, home: &Path) -> bool {
    home.is_absolute()
        && (executable == Path::new("/Applications/Gatto.app/Contents/MacOS/gatto")
            || executable == home.join("Applications/Gatto.app/Contents/MacOS/gatto"))
}

fn normalized_uuid(value: &str) -> Option<String> {
    let valid = value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
        && value.bytes().any(|byte| byte != b'0' && byte != b'-');
    valid.then(|| value.to_ascii_uppercase())
}

#[derive(Deserialize)]
struct AttachedImages {
    images: Vec<DiskImage>,
}

#[derive(Deserialize)]
struct DiskImage {
    #[serde(rename = "owner-uid")]
    owner_uid: Option<u32>,
    writeable: Option<bool>,
    #[serde(rename = "image-path")]
    image_path: Option<PathBuf>,
    #[serde(rename = "system-entities", default)]
    entities: Vec<DiskEntity>,
}

#[derive(Deserialize)]
struct DiskEntity {
    #[serde(rename = "dev-entry")]
    device: Option<String>,
    #[serde(rename = "mount-point")]
    mount: Option<PathBuf>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct VolumeInfo {
    #[serde(rename = "VolumeUUID")]
    volume_uuid: Option<String>,
    device_node: Option<String>,
    parent_whole_disk: Option<String>,
    mount_point: Option<PathBuf>,
    bus_protocol: Option<String>,
    writable: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallerDisk {
    image: PathBuf,
    whole_device: String,
    volume_device: String,
    mount: PathBuf,
}

fn is_whole_disk(device: &str) -> bool {
    device.strip_prefix("/dev/disk").is_some_and(|number| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn is_volume_device(device: &str, whole: &str) -> bool {
    device.strip_prefix(whole).is_some_and(|partition| {
        partition.strip_prefix('s').is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
    })
}

fn is_absolute_plain_path(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

fn select_installer(
    images: AttachedImages,
    uuid: &str,
    uid: u32,
    mut volume_info: impl FnMut(&str) -> Option<VolumeInfo>,
) -> Option<InstallerDisk> {
    let uuid = normalized_uuid(uuid)?;
    let mut found = None;
    let mut inspected = 0;
    for image in images.images {
        // hdiutil's descriptive image-type varies by image format and macOS
        // version. Use its writeability flag and the volume's metadata instead.
        if image.owner_uid != Some(uid) || image.writeable != Some(false) {
            continue;
        }
        let Some(image_path) = image.image_path.filter(|path| is_absolute_plain_path(path)) else {
            continue;
        };
        let whole_devices: Vec<_> = image
            .entities
            .iter()
            .filter_map(|entity| {
                entity
                    .device
                    .as_deref()
                    .filter(|device| is_whole_disk(device))
            })
            .collect();
        let [whole] = whole_devices.as_slice() else {
            continue;
        };
        for entity in &image.entities {
            let (Some(device), Some(mount)) = (&entity.device, &entity.mount) else {
                continue;
            };
            if !is_volume_device(device, whole) || !is_absolute_plain_path(mount) {
                continue;
            }
            inspected += 1;
            if inspected > MAX_VOLUMES {
                return None;
            }
            let info = volume_info(device)?;
            if info
                .volume_uuid
                .as_deref()
                .and_then(normalized_uuid)
                .as_deref()
                != Some(&uuid)
                || info.device_node.as_deref() != Some(device)
                || info.parent_whole_disk.as_deref() != whole.strip_prefix("/dev/")
                || info.mount_point.as_ref() != Some(mount)
                || info.bus_protocol.as_deref() != Some("Disk Image")
                || info.writable != Some(false)
            {
                continue;
            }
            if found.is_some() {
                // Copied DMGs retain their UUID. Never choose between two mounts.
                return None;
            }
            found = Some(InstallerDisk {
                image: image_path.clone(),
                whole_device: (*whole).to_owned(),
                volume_device: device.clone(),
                mount: mount.clone(),
            });
        }
    }
    found
}

fn discover(uuid: &str, uid: u32, deadline: Instant) -> Option<InstallerDisk> {
    let images = metadata("/usr/bin/hdiutil", &["info", "-plist"], deadline)?;
    select_installer(images, uuid, uid, |device| {
        metadata("/usr/sbin/diskutil", &["info", "-plist", device], deadline)
    })
}

fn cleanup(uuid: &str, uid: u32) -> Option<bool> {
    let deadline = Instant::now() + METADATA_TIMEOUT;
    let installer = discover(uuid, uid, deadline)?;
    // Recheck ownership, UUID, device, backing path, and ambiguity immediately
    // before ejecting. Mount paths and BSD device names can be reused.
    if discover(uuid, uid, deadline)? != installer || Instant::now() >= deadline {
        return None;
    }
    let path = installer.mount.to_str()?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    // This public API requests normal ejection and reports a busy volume as an
    // error. It does not require Finder automation or force-unmount flags.
    Some(
        NSWorkspace::sharedWorkspace()
            .unmountAndEjectDeviceAtURL_error(&url)
            .is_ok(),
    )
}

/// Read system metadata only. The backing DMG and mounted volume contents are
/// never opened. Bound subprocess lifetime/output so discovery cannot linger.
fn metadata<T: DeserializeOwned>(program: &str, args: &[&str], deadline: Instant) -> Option<T> {
    let bytes = command_output(program, args, deadline)?;
    let data = NSData::with_bytes(&bytes);
    // SAFETY: the optional format pointer is null; parsing yields an immutable
    // property list owned by Foundation. Unknown or malformed data fails closed.
    let plist = unsafe {
        NSPropertyListSerialization::propertyListWithData_options_format_error(
            &data,
            NSPropertyListMutabilityOptions::empty(),
            std::ptr::null_mut(),
        )
    }
    .ok()?;
    // SAFETY: this is a Foundation property list. Check JSON compatibility before
    // serialization, rejecting unsupported plist types rather than assuming them.
    if !unsafe { NSJSONSerialization::isValidJSONObject(&plist) } {
        return None;
    }
    let json = unsafe {
        NSJSONSerialization::dataWithJSONObject_options_error(&plist, NSJSONWritingOptions::empty())
    }
    .ok()?;
    serde_json::from_slice(&json.to_vec()).ok()
}

fn command_output(program: &str, args: &[&str], deadline: Instant) -> Option<Vec<u8>> {
    if Instant::now() >= deadline {
        return None;
    }
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_METADATA_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() as u64 <= MAX_METADATA_BYTES).then_some(bytes)
    });
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader.join().ok().flatten();
    if success { bytes } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const UUID: &str = "DBE51245-E7B0-3B49-8D64-3C9C99A41597";

    fn image() -> serde_json::Value {
        json!({
            "owner-uid": 501, "image-type": "read-only disk image", "writeable": false,
            "image-path": "/Users/test/Downloads/gatto.dmg",
            "system-entities": [
                {"dev-entry": "/dev/disk5"},
                {"dev-entry": "/dev/disk5s1", "mount-point": "/Volumes/Install Gatto"}
            ]
        })
    }

    fn info() -> VolumeInfo {
        VolumeInfo {
            volume_uuid: Some(UUID.to_owned()),
            device_node: Some("/dev/disk5s1".into()),
            parent_whole_disk: Some("disk5".into()),
            mount_point: Some("/Volumes/Install Gatto".into()),
            bus_protocol: Some("Disk Image".into()),
            writable: Some(false),
        }
    }

    fn select(
        values: Vec<serde_json::Value>,
        query: impl FnMut(&str) -> Option<VolumeInfo>,
    ) -> Option<InstallerDisk> {
        select_installer(
            serde_json::from_value(json!({"images": values})).unwrap(),
            UUID,
            501,
            query,
        )
    }

    #[test]
    fn only_installed_executables_are_eligible() {
        let home = Path::new("/Users/test");
        for installed in [
            "/Applications/Gatto.app",
            "/Users/test/Applications/Gatto.app",
        ] {
            assert!(is_installed_executable(
                &Path::new(installed).join("Contents/MacOS/gatto"),
                home
            ));
        }
        for other in [
            "/Volumes/Install Gatto/Gatto.app",
            "/private/var/folders/random/AppTranslocation/Gatto.app",
            "/Users/other/Applications/Gatto.app",
            "/Applications/Other.app",
        ] {
            assert!(!is_installed_executable(
                &Path::new(other).join("Contents/MacOS/gatto"),
                home
            ));
        }
        assert!(!is_installed_executable(
            Path::new("/repo/target/debug/gatto"),
            home
        ));
    }

    #[test]
    fn invalid_or_missing_installer_identity_fails_closed() {
        for invalid in [
            "",
            "unknown",
            "00000000-0000-0000-0000-000000000000",
            "DBE51245/E7B0-3B49-8D64-3C9C99A41597",
        ] {
            assert!(normalized_uuid(invalid).is_none());
        }
        assert_eq!(
            normalized_uuid(&UUID.to_ascii_lowercase()).as_deref(),
            Some(UUID)
        );
        assert!(select(vec![], |_| unreachable!()).is_none());
    }

    #[test]
    fn matches_uuid_and_device_metadata_without_reading_image_files() {
        for label in ["read-only disk image", "UDIF read-only compressed (zlib)"] {
            let mut candidate = image();
            candidate["image-type"] = json!(label);
            let selected = select(vec![candidate], |device| {
                assert_eq!(device, "/dev/disk5s1");
                Some(info())
            })
            .unwrap();
            assert_eq!(selected.mount, Path::new("/Volumes/Install Gatto"));
            assert_eq!(selected.whole_device, "/dev/disk5");
        }
    }

    #[test]
    fn duplicate_uuid_mounts_are_ambiguous() {
        assert!(select(vec![image(), image()], |_| Some(info())).is_none());
    }

    #[test]
    fn foreign_writable_or_malformed_images_are_not_inspected() {
        for (field, value) in [
            ("owner-uid", json!(502)),
            ("owner-uid", json!(null)),
            ("writeable", json!(true)),
            ("image-path", json!("relative.dmg")),
        ] {
            let mut candidate = image();
            candidate[field] = value;
            assert!(select(vec![candidate], |_| panic!("ineligible image inspected")).is_none());
        }
        let mut candidate = image();
        candidate["system-entities"][1]["dev-entry"] = json!("/dev/disk50s1");
        assert!(select(vec![candidate], |_| panic!("wrong disk inspected")).is_none());
    }

    #[test]
    fn changed_or_unavailable_volume_metadata_skips_cleanup() {
        assert!(select(vec![image()], |_| None).is_none());
        for field in 0..6 {
            assert!(
                select(vec![image()], |_| {
                    let mut metadata = info();
                    match field {
                        0 => {
                            metadata.volume_uuid =
                                Some("ABE51245-E7B0-3B49-8D64-3C9C99A41597".into())
                        }
                        1 => metadata.device_node = Some("/dev/disk6s1".into()),
                        2 => metadata.parent_whole_disk = Some("disk6".into()),
                        3 => metadata.mount_point = Some("/Volumes/Other".into()),
                        4 => metadata.bus_protocol = Some("USB".into()),
                        _ => metadata.writable = None,
                    }
                    Some(metadata)
                })
                .is_none()
            );
        }
    }

    #[test]
    fn discovery_subprocesses_are_bounded() {
        let started = Instant::now();
        assert!(
            command_output("/bin/sleep", &["30"], started + Duration::from_millis(100)).is_none()
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
