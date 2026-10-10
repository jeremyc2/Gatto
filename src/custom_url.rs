use thiserror::Error;
use url::Url;

pub const SCHEME: &str = "gatto";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomUrlAction {
    Open,
    Preview,
    QuickCopy,
    Settings,
}

impl CustomUrlAction {
    pub fn parse(value: &str) -> Result<Self, CustomUrlError> {
        let url = Url::parse(value).map_err(|_| CustomUrlError::InvalidUrl)?;
        if url.scheme() != SCHEME {
            return Err(CustomUrlError::UnsupportedScheme);
        }
        if url.query().is_some() || url.fragment().is_some() {
            return Err(CustomUrlError::UnexpectedComponents);
        }

        let action = if let Some(host) = url.host_str() {
            if !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
                || !matches!(url.path(), "" | "/")
            {
                return Err(CustomUrlError::UnexpectedComponents);
            }
            host
        } else {
            let path = url.path().trim_matches('/');
            if path.is_empty() {
                return Err(CustomUrlError::MissingAction);
            }
            if path.contains('/') {
                return Err(CustomUrlError::UnexpectedComponents);
            }
            path
        };

        match action {
            "open" => Ok(Self::Open),
            "preview" => Ok(Self::Preview),
            "quick-copy" => Ok(Self::QuickCopy),
            "settings" => Ok(Self::Settings),
            _ => Err(CustomUrlError::UnsupportedAction),
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CustomUrlError {
    #[error("The URL is not valid.")]
    InvalidUrl,
    #[error("Only gatto:// URLs are supported.")]
    UnsupportedScheme,
    #[error("The Gatto URL does not name an action.")]
    MissingAction,
    #[error("The Gatto URL contains unsupported components.")]
    UnexpectedComponents,
    #[error("The Gatto URL names an unsupported action.")]
    UnsupportedAction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_actions() {
        assert_eq!(
            CustomUrlAction::parse("gatto://open"),
            Ok(CustomUrlAction::Open)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://preview"),
            Ok(CustomUrlAction::Preview)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://quick-copy"),
            Ok(CustomUrlAction::QuickCopy)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://settings"),
            Ok(CustomUrlAction::Settings)
        );
    }

    #[test]
    fn accepts_path_style_urls_from_automation_tools() {
        assert_eq!(
            CustomUrlAction::parse("gatto:quick-copy"),
            Ok(CustomUrlAction::QuickCopy)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto:///preview"),
            Ok(CustomUrlAction::Preview)
        );
    }

    #[test]
    fn rejects_unknown_or_ambiguous_urls() {
        assert_eq!(
            CustomUrlAction::parse("https://quick-copy"),
            Err(CustomUrlError::UnsupportedScheme)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://"),
            Err(CustomUrlError::MissingAction)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://upload"),
            Err(CustomUrlError::UnsupportedAction)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://quick-copy?repository=other"),
            Err(CustomUrlError::UnexpectedComponents)
        );
        assert_eq!(
            CustomUrlAction::parse("gatto://quick-copy/again"),
            Err(CustomUrlError::UnexpectedComponents)
        );
    }
}
