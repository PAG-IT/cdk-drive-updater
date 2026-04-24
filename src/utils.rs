use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, Timelike};
use version_compare::Cmp;

pub(crate) const NOT_FOUND_COMPACT: &str = "NotFound";
pub(crate) const NOT_FOUND_DISPLAY: &str = "Not Found";

pub(crate) fn cwd_child(name: &str) -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(name)
}

pub(crate) fn exe_dir_child(name: &str) -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(name)
}

pub(crate) fn env_path_or_else(var_name: &str, default: impl FnOnce() -> PathBuf) -> PathBuf {
    non_empty_env_var(var_name)
        .map(PathBuf::from)
        .unwrap_or_else(default)
}

pub(crate) fn non_empty_env_var(var_name: &str) -> Option<String> {
    env::var(var_name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn compare_versions(left: &str, right: &str) -> Ordering {
    match version_compare::compare(left, right) {
        Ok(Cmp::Lt) => Ordering::Less,
        Ok(Cmp::Gt) => Ordering::Greater,
        Ok(Cmp::Eq) => Ordering::Equal,
        _ => left.trim().cmp(right.trim()),
    }
}

pub(crate) fn is_missing_value(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case(NOT_FOUND_COMPACT)
        || trimmed.eq_ignore_ascii_case(NOT_FOUND_DISPLAY)
}

pub(crate) fn replace_file(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    delete_if_exists(path);
    fs::write(path, contents).with_context(|| format!("failed to write file: {}", path.display()))
}

pub(crate) fn delete_if_exists(path: &Path) {
    match path.try_exists() {
        Ok(true) => {
            if let Err(error) = fs::remove_file(path) {
                log::warn!(
                    "Failed to delete existing file | path={} | error={}",
                    path.display(),
                    error
                );
            }
        }
        Ok(false) => {}
        Err(error) => {
            log::warn!(
                "Could not check existence of file | path={} | error={}",
                path.display(),
                error
            );
        }
    }
}

pub(crate) fn safe_filename_token(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut last_was_underscore = false;

    for c in name.chars() {
        if c.is_alphanumeric() || c == '.' || c == '-' {
            result.push(c);
            last_was_underscore = false;
        } else if !last_was_underscore {
            result.push('_');
            last_was_underscore = true;
        }
    }

    result.trim_matches('_').to_string().to_ascii_lowercase()
}

pub(crate) fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub(crate) fn build_timestamp(now: chrono::DateTime<Local>) -> String {
    let hour_24 = now.hour();
    let hour_12 = match hour_24 % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if hour_24 < 12 { "am" } else { "pm" };

    format!(
        "{}--{}-{:02}-{}",
        now.format("%Y-%m-%d"),
        hour_12,
        now.minute(),
        meridiem
    )
}
