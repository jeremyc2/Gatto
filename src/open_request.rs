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
                    } else if let Ok(path) = url.to_file_path() {
                        files.push(path);
                    } else {
                        requests.push(Self::Invalid("Gatto can only open local files. Copy the file to this Mac and try again.".into()));
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
        let paths = ["/tmp/a space.png", "/tmp/猫 #?%\n$(touch nope).mov"];
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
        let requests = OpenRequest::from_urls(
            [
                "file:///tmp/a.png",
                "file://server/share/b.png",
                "gatto://app-preferences",
                "file:///tmp/b.mp4",
                "file:///tmp/a.png?upload=true",
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
            OpenRequest::Files(vec!["/tmp/a.png".into(), "/tmp/b.mp4".into()])
        );
    }
}
