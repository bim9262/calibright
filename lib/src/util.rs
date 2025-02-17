use std::path::Path;

use futures_util::future::join_all;
use serde::de::DeserializeOwned;
use std::future::Future;
use tokio::io::AsyncReadExt as _;

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

pub async fn deserialize_toml_file<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let contents = read_file(path).await?;

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
        CalibrightError::Other(format!(
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
        Err(CalibrightError::InvalidScaleParameters { low, high })
    } else {
        Ok(absolute_value.clamp(0.0, 1.0) * (high - low) + low)
    }
}

// Scale a number from an arbitrary scale to 0.0-1.0
pub fn scale_to_clamped_absolute(relative_value: f64, low: f64, high: f64) -> Result<f64> {
    if low > high {
        Err(CalibrightError::InvalidScaleParameters { low, high })
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
    let mut error = CalibrightError::NoDevices;
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
