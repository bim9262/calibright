use std::path::{Path, PathBuf};

use dirs::config_dir;
use futures::{future::join_all, Future};
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;

use crate::errors::*;

macro_rules! make_log_macro {
    (@wdoll $macro_name:ident, $block_name:literal, ($dol:tt)) => {
        #[allow(dead_code)]
        macro_rules! $macro_name {
            ($dol($args:tt)+) => {
                ::log::$macro_name!(target: $block_name, $dol($args)+);
            };
        }
    };
    ($macro_name:ident, $block_name:literal) => {
        make_log_macro!(@wdoll $macro_name, $block_name, ($));
    };
}

/// Tries to find a file in standard locations:
/// - Fist try to find a file by full path
/// - Then try XDG_CONFIG_HOME (e.g. `~/.config`)
///
/// Automatically append an extension if not presented.
pub fn find_file(file: &str, subdir: Option<&str>, extension: Option<&str>) -> Option<PathBuf> {
    let file = PathBuf::from(file);
    if file.exists() {
        return Some(file);
    }

    // Try XDG_CONFIG_HOME (e.g. `~/.config`)
    if let Some(mut xdg_config) = config_dir() {
        xdg_config.push("calibright");
        if let Some(subdir) = subdir {
            xdg_config.push(subdir);
        }
        xdg_config.push(&file);
        if let Some(file) = exists_with_opt_extension(&xdg_config, extension) {
            return Some(file);
        }
    }

    None
}

fn exists_with_opt_extension(file: &Path, extension: Option<&str>) -> Option<PathBuf> {
    if file.exists() {
        return Some(file.into());
    }
    // If file has no extension, test with given extension
    if let (None, Some(extension)) = (file.extension(), extension) {
        let file = file.with_extension(extension);
        // Check again with extension added
        if file.exists() {
            return Some(file);
        }
    }
    None
}

pub fn deserialize_toml_file<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let contents = std::fs::read_to_string(path)
        .or_error(|| format!("Failed to read file: {}", path.display()))?;

    toml::from_str(&contents).map_err(|err| {
        #[allow(deprecated)]
        let location_msg = err
            .span()
            .map(|span| {
                let line = 1 + contents.as_bytes()[..(span.start)]
                    .iter()
                    .filter(|b| **b == b'\n')
                    .count();
                format!(" at line {line}")
            })
            .unwrap_or_default();
        Error::new(format!(
            "Failed to deserialize TOML file {}{}: {}",
            path.display(),
            location_msg,
            err.message()
        ))
    })
}

pub async fn read_file(path: impl AsRef<Path>) -> std::io::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut content = String::new();
    file.read_to_string(&mut content).await?;
    Ok(content.trim_end().to_string())
}

/// Scale a number from 0.0-1.0 to an arbitrary scale
pub fn scale_to_clamped_relative(absolute_value: f64, low: f64, high: f64) -> Result<f64> {
    if low > high {
        Err(Error::new("Low cannot be less than high"))
    } else {
        Ok(absolute_value.clamp(0.0, 1.0) * (high - low) + low)
    }
}

// Scale a number from an arbitrary scale to 0.0-1.0
pub fn scale_to_clamped_absolute(relative_value: f64, low: f64, high: f64) -> Result<f64> {
    if low > high {
        Err(Error::new("Low cannot be less than high"))
    } else {
        Ok((relative_value.clamp(low, high) - low) / (high - low))
    }
}

pub async fn join_all_accept_single_ok<I, T>(iter: I) -> Result<Vec<T>>
where
    I: IntoIterator,
    I::Item: Future<Output = Result<T>>,
{
    let all_results = join_all(iter).await;
    let mut results: Vec<T> = Vec::new();
    let mut error: Error = Error::new("tried to join 0 futures");
    for result in all_results {
        match result {
            Ok(x) => results.push(x),
            Err(e) => error = e,
        }
    }
    if results.is_empty() {
        Err(error)
    } else {
        Ok(results)
    }
}
