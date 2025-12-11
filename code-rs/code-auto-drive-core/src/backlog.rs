//! 外部记忆 Backlog 管理（ai/feature_list.json）。
//!
//! 提供 foreman 兼容的 Feature 结构体、加载/保存、验证更新，以及根据 git diff
//! 粗粒度推导受影响特性。

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// TDD 模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TddMode {
    Strict,
    Relaxed,
}

impl Default for TddMode {
    fn default() -> Self {
        Self::Relaxed
    }
}

/// 测试需求列表。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRequirements {
    #[serde(default)]
    pub unit: Vec<String>,
    #[serde(default)]
    pub e2e: Vec<String>,
}

/// 单个特性定义（foreman 兼容字段）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub module: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub test_requirements: TestRequirements,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_version")]
    pub version: i32,
    #[serde(default)]
    pub tdd_mode: TddMode,
    #[serde(default)]
    pub verification: Option<VerificationResult>,
}

fn default_version() -> i32 {
    1
}

/// 验证结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    #[serde(default)]
    pub tests_run: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub reason: Option<String>,
}

impl VerificationResult {
    pub fn new(verified: bool, tests_run: Vec<String>, summary: impl Into<String>) -> Self {
        Self {
            verified,
            tests_run,
            summary: summary.into(),
            timestamp: Utc::now().to_rfc3339(),
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Backlog 管理器。
pub struct BacklogManager {
    path: PathBuf,
    features: Vec<Feature>,
}

impl BacklogManager {
    /// 创建并尝试从文件加载。
    pub fn load(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let features = if path.exists() {
            let data = fs::read_to_string(&path)?;
            serde_json::from_str::<FeatureList>(&data)
                .map(|f| f.features)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self { path, features })
    }

    /// 直接用给定特性构造（主要用于测试或预加载）。
    pub fn from_features(path: impl Into<PathBuf>, features: Vec<Feature>) -> Self {
        Self {
            path: path.into(),
            features,
        }
    }

    fn save_internal(&self, features: &[Feature]) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = FeatureList {
            features: features.to_vec(),
        };
        let json = serde_json::to_string_pretty(&payload)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// 保存当前特性列表（缺文件时创建）。
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_internal(&self.features)
    }

    /// 返回内存中的特性列表。
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// 更新验证结果并写回文件；不存在的特性返回 Err。
    pub fn update_verification(
        &mut self,
        feature_id: &str,
        result: VerificationResult,
    ) -> anyhow::Result<()> {
        let mut updated = false;
        for feature in &mut self.features {
            if feature.id == feature_id {
                feature.verification = Some(result);
                updated = true;
                break;
            }
        }

        if !updated && !feature_id.is_empty() {
            anyhow::bail!("feature {feature_id} not found");
        }

        self.save()
    }

    /// 基于路径或标签匹配受影响特性。
    pub fn get_affected_by_diff(&self, paths: &[impl AsRef<Path>]) -> Vec<Feature> {
        let mut affected = Vec::new();
        for feature in &self.features {
            let module = feature.module.as_str();
            let mut matched = false;
            for path in paths {
                let path = path.as_ref().to_string_lossy().to_string();
                if !module.is_empty() && path.starts_with(module) {
                    matched = true;
                    break;
                }
                if feature
                    .tags
                    .iter()
                    .any(|tag| !tag.is_empty() && path.contains(tag))
                {
                    matched = true;
                    break;
                }
            }
            if matched {
                affected.push(feature.clone());
            }
        }
        affected
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct FeatureList {
    pub features: Vec<Feature>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn round_trip_preserves_structure() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("feature_list.json");

        let feature = Feature {
            id: "F-1".to_string(),
            description: "desc".to_string(),
            module: "code-auto-drive-core".to_string(),
            priority: "P1".to_string(),
            status: "todo".to_string(),
            acceptance: vec!["works".to_string()],
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            tags: vec!["auto-drive".to_string()],
            version: 1,
            tdd_mode: TddMode::Strict,
            verification: None,
        };

        let mgr = BacklogManager {
            path: path.clone(),
            features: vec![feature.clone()],
        };
        mgr.save().unwrap();

        let loaded = BacklogManager::load(path).unwrap();
        assert_eq!(loaded.features(), &[feature]);
    }

    #[allow(unused_mut)]
    #[test]
    fn update_verification_writes_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("feature_list.json");
        let mut mgr = BacklogManager::load(path.clone()).unwrap();
        mgr.features.push(Feature {
            id: "F-2".to_string(),
            description: "demo".to_string(),
            ..Default::default()
        });
        mgr.save().unwrap();

        mgr.update_verification(
            "F-2",
            VerificationResult::new(true, vec!["unit".to_string()], "ok"),
        )
        .unwrap();

        let data = fs::read_to_string(path).unwrap();
        assert!(data.contains("F-2"));
        assert!(data.contains("verified"));
    }

    #[test]
    fn affected_features_match_module_or_tag() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("feature_list.json");
        let mgr = BacklogManager {
            path,
            features: vec![
                Feature {
                    id: "F-1".to_string(),
                    description: "core".to_string(),
                    module: "core/src".to_string(),
                    ..Default::default()
                },
                Feature {
                    id: "F-2".to_string(),
                    description: "tagged".to_string(),
                    tags: vec!["ui".to_string()],
                    ..Default::default()
                },
            ],
        };

        let affected = mgr.get_affected_by_diff(&["core/src/lib.rs", "docs/ui.md"]);
        let ids: Vec<String> = affected.into_iter().map(|f| f.id).collect();
        assert!(ids.contains(&"F-1".to_string()));
        assert!(ids.contains(&"F-2".to_string()));
    }
}
