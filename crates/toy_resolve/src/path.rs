//! Module path representation and resolution

use crate::error::PathError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModulePath {
    /// Standard library: std.io
    Std(Vec<String>),
    /// Package: pkg.http
    Package(Vec<String>),
    /// Relative: .helpers, ..utils
    Relative { up: usize, segments: Vec<String> },
    /// Absolute: src.utils.helpers
    Absolute(Vec<String>),
}

impl ModulePath {
    /// Create a new standard library path
    pub fn std(segments: Vec<String>) -> Self {
        Self::Std(segments)
    }

    /// Create a new package path
    pub fn package(segments: Vec<String>) -> Self {
        Self::Package(segments)
    }

    /// Create a new relative path
    pub fn relative(up: usize, segments: Vec<String>) -> Self {
        Self::Relative { up, segments }
    }

    /// Create a new absolute path
    pub fn absolute(segments: Vec<String>) -> Self {
        Self::Absolute(segments)
    }

    /// Resolve relative path to absolute path
    pub fn resolve_relative_to(&self, base: &ModulePath) -> Result<ModulePath, PathError> {
        match self {
            ModulePath::Relative { up, segments } => {
                // Get base segments
                let base_segments = match base {
                    ModulePath::Std(s) => s,
                    ModulePath::Package(s) => s,
                    ModulePath::Absolute(s) => s,
                    ModulePath::Relative { .. } => {
                        return Err(PathError::InvalidPath);
                    }
                };

                // Go up `up` levels
                if *up > base_segments.len() {
                    return Err(PathError::TooManyParents);
                }

                let mut resolved = base_segments[..base_segments.len() - up].to_vec();
                resolved.extend(segments.clone());

                Ok(ModulePath::Absolute(resolved))
            }
            _ => Ok(self.clone()),
        }
    }

    /// Convert to file system path
    pub fn to_fs_path(&self, project_root: &Path) -> PathBuf {
        let segments = match self {
            ModulePath::Std(s) => {
                let mut path = project_root.join("std");
                for seg in s {
                    path = path.join(seg);
                }
                return path.with_extension("toy");
            }
            ModulePath::Package(s) => {
                let mut path = project_root.join("pkg");
                for seg in s {
                    path = path.join(seg);
                }
                return path.with_extension("toy");
            }
            ModulePath::Absolute(s) => s,
            ModulePath::Relative { .. } => {
                panic!("Cannot convert relative path to fs path without base");
            }
        };

        let mut path = project_root.to_path_buf();
        for seg in segments {
            path = path.join(seg);
        }
        path.with_extension("toy")
    }

    /// Get segments for this path
    pub fn segments(&self) -> &[String] {
        match self {
            ModulePath::Std(s) | ModulePath::Package(s) | ModulePath::Absolute(s) => s,
            ModulePath::Relative { segments, .. } => segments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_resolution() {
        let base = ModulePath::absolute(vec!["src".into(), "utils".into()]);
        let rel = ModulePath::relative(1, vec!["helpers".into()]);

        let resolved = rel.resolve_relative_to(&base).unwrap();
        assert_eq!(
            resolved,
            ModulePath::absolute(vec!["src".into(), "helpers".into()])
        );
    }

    #[test]
    fn test_relative_too_many_parents() {
        let base = ModulePath::absolute(vec!["src".into()]);
        let rel = ModulePath::relative(2, vec!["helpers".into()]);

        assert!(matches!(
            rel.resolve_relative_to(&base),
            Err(PathError::TooManyParents)
        ));
    }
}
