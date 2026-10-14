use std::{
    fmt,
    path::{Path, PathBuf},
};

use async_trait::async_trait;

/// Implemented by the engine: renders one frame and writes the viewport to a PNG.
#[async_trait]
pub trait ScreenshotProvider: Send + Sync {
    /// The absolute path of the written image.
    async fn capture(&self) -> Result<PathBuf, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotError {
    /// No engine is running, so nothing can be captured.
    NoEngine,
    /// The engine failed or answered with a path that is not a file.
    Failed(String),
}

impl fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEngine => write!(f, "no engine is running"),
            Self::Failed(detail) => write!(f, "screenshot failed: {detail}"),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// Asks the engine for a screenshot and checks that what it answers is a file the shell can read.
pub async fn capture_screenshot(
    provider: Option<&dyn ScreenshotProvider>,
) -> Result<PathBuf, ScreenshotError> {
    let provider = provider.ok_or(ScreenshotError::NoEngine)?;
    let path = provider.capture().await.map_err(ScreenshotError::Failed)?;
    check_written(&path).await?;
    Ok(path)
}

async fn check_written(path: &Path) -> Result<(), ScreenshotError> {
    if !path.is_absolute() {
        return Err(ScreenshotError::Failed(format!(
            "{} is not an absolute path",
            path.display()
        )));
    }
    match tokio::fs::metadata(path).await {
        Ok(meta) if meta.is_file() && meta.len() > 0 => Ok(()),
        Ok(_) => Err(ScreenshotError::Failed(format!(
            "{} is empty or not a file",
            path.display()
        ))),
        Err(e) => Err(ScreenshotError::Failed(format!("{}: {e}", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Result<PathBuf, String>);

    #[async_trait]
    impl ScreenshotProvider for Fixed {
        async fn capture(&self) -> Result<PathBuf, String> {
            self.0.clone()
        }
    }

    #[tokio::test]
    async fn no_provider_means_no_engine() {
        assert_eq!(
            capture_screenshot(None).await,
            Err(ScreenshotError::NoEngine)
        );
    }

    #[tokio::test]
    async fn a_written_file_is_returned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.png");
        std::fs::write(&path, b"png").unwrap();
        let provider = Fixed(Ok(path.clone()));
        assert_eq!(capture_screenshot(Some(&provider)).await, Ok(path));
    }

    #[tokio::test]
    async fn the_engine_error_passes_through() {
        let provider = Fixed(Err("no viewport".into()));
        assert_eq!(
            capture_screenshot(Some(&provider)).await,
            Err(ScreenshotError::Failed("no viewport".into()))
        );
    }

    #[tokio::test]
    async fn missing_empty_and_relative_paths_are_failures() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.png");
        std::fs::write(&empty, b"").unwrap();
        for path in [
            dir.path().join("missing.png"),
            empty,
            PathBuf::from("rel.png"),
        ] {
            let provider = Fixed(Ok(path));
            assert!(matches!(
                capture_screenshot(Some(&provider)).await,
                Err(ScreenshotError::Failed(_))
            ));
        }
    }
}
