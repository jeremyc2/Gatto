//! Preserve each macOS open event as one selection, including on cold launch.
use std::path::PathBuf;

use crate::custom_url::CustomUrlAction;
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum OpenRequest {
    Files(Vec<PathBuf>),
    Action(CustomUrlAction),
    Invalid(String),
}

impl OpenRequest {
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
