//! 进度日志记录（ai/progress.log）。
//!
//! 记录 STEP/CHANGE/VERIFY/REPLAN 事件，格式：
//! `timestamp | type | status | tests | summary | note`

use std::fs::OpenOptions;
use std::fs::{self};
use std::io::Write;
use std::path::PathBuf;

use chrono::Utc;

/// 进度类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressType {
    Step,
    Change,
    Verify,
    Replan,
}

impl ProgressType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Step => "STEP",
            Self::Change => "CHANGE",
            Self::Verify => "VERIFY",
            Self::Replan => "REPLAN",
        }
    }
}

/// 日志条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEntry {
    pub kind: ProgressType,
    pub status: String,
    pub tests: String,
    pub summary: String,
    pub note: String,
}

impl ProgressEntry {
    pub fn new(
        kind: ProgressType,
        status: impl Into<String>,
        tests: impl Into<String>,
        summary: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            status: status.into(),
            tests: tests.into(),
            summary: summary.into(),
            note: note.into(),
        }
    }
}

/// 进度日志器。
pub struct ProgressLogger {
    path: PathBuf,
}

impl ProgressLogger {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 追加一条记录，缺文件则创建。
    pub fn append(&self, entry: ProgressEntry) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let line = format!(
            "{} | {} | {} | {} | {} | {}\n",
            Utc::now().to_rfc3339(),
            entry.kind.as_str(),
            entry.status,
            entry.tests,
            entry.summary,
            entry.note
        );
        file.write_all(line.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn writes_expected_line_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("progress.log");
        let logger = ProgressLogger::new(path.clone());

        logger
            .append(ProgressEntry::new(
                ProgressType::Step,
                "running",
                "cargo test",
                "dispatch",
                "note",
            ))
            .unwrap();

        let data = fs::read_to_string(path).unwrap();
        let parts: Vec<&str> = data.trim().split('|').collect();
        assert_eq!(parts.len(), 6);
        assert!(parts[1].trim().starts_with("STEP"));
    }
}
