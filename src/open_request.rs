//! Keep each desktop open event together, including on cold launch.
use std::path::PathBuf;

use crate::custom_url::CustomUrlAction;
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum OpenRequest {
    #[cfg(windows)]
    Reopen,
    Files(Vec<PathBuf>),
    Action(CustomUrlAction),
    Invalid(String),
}

impl OpenRequest {
    #[cfg(windows)]
    pub fn from_windows_args(args: &[std::ffi::OsString]) -> Vec<Self> {
        if args.is_empty() {
            return vec![Self::Reopen];
        }
        // Explorer and protocol handlers use separate modes. A file name is
        // never interpreted as a Gatto URL or as a background-launch switch.
        if args[0] == "--url" {
            return vec![match &args[1..] {
                [link] => match link.to_str().map(CustomUrlAction::parse) {
                    Some(Ok(action)) => Self::Action(action),
                    Some(Err(error)) => Self::Invalid(error.to_string()),
                    None => Self::Invalid("The Gatto URL is not valid Unicode.".into()),
                },
                _ => Self::Invalid("Expected one Gatto URL.".into()),
            }];
        }
        let paths = if args[0] == "--files" {
            &args[1..]
        } else {
            args
        };
        let paths: Vec<_> = paths.iter().map(PathBuf::from).collect();
        use std::path::{Component, Prefix};
        if paths.is_empty() || paths.iter().any(|path| {
            !path.is_absolute() || !matches!(
                path.components().next(),
                Some(Component::Prefix(drive)) if matches!(drive.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            )
        }) {
            return vec![Self::Invalid("Gatto can only open files on a local drive. Copy the files locally and try again.".into())];
        }
        vec![Self::Files(paths)]
    }

    pub fn from_urls(urls: Vec<String>) -> Vec<Self> {
        let mut requests = Vec::new();
        let mut files = Vec::new();
        for value in urls {
            match Url::parse(&value) {
                Ok(url) if url.scheme() == "file" => {
                    if url.query().is_some() || url.fragment().is_some() {
                        requests.push(Self::Invalid(
                            "The file URL contains unsupported components.".into(),
                        ));
                    } else if let Ok(path) = url.to_file_path()
                        && matches!(url.host_str(), None | Some("localhost"))
                    {
                        files.push(path);
                    } else {
                        requests.push(Self::Invalid("Gatto can only open local files. Copy the file to this computer and try again.".into()));
                    }
                }
                _ => requests.push(match CustomUrlAction::parse(&value) {
                    Ok(action) => Self::Action(action),
                    Err(error) => Self::Invalid(error.to_string()),
                }),
            }
        }
        if !files.is_empty() {
            requests.push(Self::Files(files));
        }
        requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_activation_keeps_file_selections_separate_from_urls() {
        fn request(args: &[&str]) -> Vec<OpenRequest> {
            OpenRequest::from_windows_args(
                &args
                    .iter()
                    .map(std::ffi::OsString::from)
                    .collect::<Vec<_>>(),
            )
        }
        assert_eq!(request(&[]), vec![OpenRequest::Reopen]);
        assert_eq!(
            request(&["--files", r"C:\a space.png", r"C:\猫 #%&.mp4"]),
            vec![OpenRequest::Files(vec![
                r"C:\a space.png".into(),
                r"C:\猫 #%&.mp4".into()
            ])]
        );
        assert_eq!(
            request(&["--url", "gatto://open"]),
            vec![OpenRequest::Action(CustomUrlAction::Open)]
        );
        assert!(matches!(
            request(&["--url", "gatto://upload"]).as_slice(),
            [OpenRequest::Invalid(_)]
        ));
        assert!(matches!(
            request(&["--files", "gatto://open"]).as_slice(),
            [OpenRequest::Invalid(_)]
        ));
    }

    #[test]
    fn preserves_one_selection_and_decodes_paths_without_shell_interpretation() {
        #[cfg(not(windows))]
        let paths = ["/tmp/a space.png", "/tmp/猫 #?%\n$(touch nope).mov"];
        #[cfg(windows)]
        let paths = [r"C:\Temp\a space.png", r"C:\Temp\猫 #%$(touch nope).mov"];
        let urls = paths
            .iter()
            .map(|path| Url::from_file_path(path).unwrap().into())
            .collect();
        assert_eq!(
            OpenRequest::from_urls(urls),
            vec![OpenRequest::Files(paths.map(PathBuf::from).into())]
        );
    }

    #[test]
    fn invalid_urls_do_not_discard_local_files_or_change_custom_actions() {
        #[cfg(not(windows))]
        let paths = ["/tmp/a.png", "/tmp/b.mp4"];
        #[cfg(windows)]
        let paths = [r"C:\Temp\a.png", r"C:\Temp\b.mp4"];
        let urls = paths.map(|path| Url::from_file_path(path).unwrap().to_string());
        let requests = OpenRequest::from_urls(
            [
                &urls[0],
                "file://server/share/b.png",
                "gatto://app-preferences",
                &urls[1],
                &format!("{}?upload=true", urls[0]),
                "https://example.com/a.png",
            ]
            .map(str::to_owned)
            .into(),
        );
        assert!(matches!(&requests[0], OpenRequest::Invalid(_)));
        assert_eq!(
            requests[1],
            OpenRequest::Action(CustomUrlAction::AppPreferences)
        );
        assert!(matches!(&requests[2], OpenRequest::Invalid(_)));
        assert!(matches!(&requests[3], OpenRequest::Invalid(_)));
        assert_eq!(
            requests[4],
            OpenRequest::Files(paths.map(PathBuf::from).into())
        );
    }
}
