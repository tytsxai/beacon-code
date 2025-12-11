//! 选择性测试与 TDD 规划。
//!
//! - 从 git diff 解析变更路径
//! - 将变更映射到 Backlog 特性及测试需求
//! - 根据 TDD 模式生成测试计划，支持 sandbox 下跳过网络/e2e

use std::env;
use std::path::PathBuf;

use crate::backlog::BacklogManager;
use crate::backlog::Feature;
use crate::backlog::TddMode;
use crate::backlog::VerificationResult;

/// 测试计划类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPlan {
    pub quick: Vec<String>,
    pub full: Vec<String>,
    pub missing: Vec<String>,
}

impl TestPlan {
    pub fn empty() -> Self {
        Self {
            quick: Vec::new(),
            full: Vec::new(),
            missing: Vec::new(),
        }
    }
}

/// 解析 git diff 输出为路径列表。
pub fn parse_git_diff_output(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| PathBuf::from(l.trim()))
        .collect()
}

/// 基于受影响特性生成快速测试计划。
pub fn generate_quick_plan(affected: &[Feature]) -> TestPlan {
    generate_quick_plan_with_sandbox(affected, sandbox_mode())
}

fn generate_quick_plan_with_sandbox(affected: &[Feature], sandbox: bool) -> TestPlan {
    let mut plan = TestPlan::empty();

    for feature in affected {
        let strict_without_unit =
            feature.tdd_mode == TddMode::Strict && feature.test_requirements.unit.is_empty();
        let e2e_only =
            feature.test_requirements.unit.is_empty() && !feature.test_requirements.e2e.is_empty();

        if strict_without_unit && (e2e_only && sandbox || feature.test_requirements.e2e.is_empty())
        {
            plan.missing.push(feature.id.clone());
        }

        plan.quick
            .extend(feature.test_requirements.unit.iter().cloned());

        if !sandbox {
            plan.full
                .extend(feature.test_requirements.e2e.iter().cloned());
        }
    }

    if plan.full.is_empty() && !sandbox {
        plan.full.push("cargo test --all-features".to_string());
    }

    plan
}

/// 汇总：从 git diff + backlog 路径生成测试计划。
pub fn plan_from_diff(backlog: &BacklogManager, diff_output: &str) -> TestPlan {
    let paths = parse_git_diff_output(diff_output);
    let affected = backlog.get_affected_by_diff(&paths);
    generate_quick_plan(&affected)
}

/// 单个测试命令的执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommandResult {
    pub command: String,
    pub passed: bool,
    pub output: Option<String>,
}

impl TestCommandResult {
    pub fn success(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            passed: true,
            output: None,
        }
    }

    pub fn failure(command: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            passed: false,
            output: Some(output.into()),
        }
    }
}

/// 根据测试计划与执行结果生成验证结果（包含严格 TDD 检查）。
pub fn verification_result_for_feature(
    feature: &Feature,
    plan: &TestPlan,
    executed: &[TestCommandResult],
    summary: impl Into<String>,
) -> VerificationResult {
    verification_result_for_feature_with_env(feature, plan, executed, summary, sandbox_mode())
}

fn sandbox_mode() -> bool {
    env::var("CODEX_SANDBOX_NETWORK_DISABLED").is_ok()
}

pub fn verification_result_for_feature_with_env(
    feature: &Feature,
    plan: &TestPlan,
    executed: &[TestCommandResult],
    summary: impl Into<String>,
    sandbox: bool,
) -> VerificationResult {
    let missing_due_to_plan = plan.missing.contains(&feature.id);
    let e2e_only =
        feature.test_requirements.unit.is_empty() && !feature.test_requirements.e2e.is_empty();
    let strict_with_no_runnable_tests = feature.tdd_mode == TddMode::Strict
        && (feature.test_requirements.unit.is_empty()
            && (feature.test_requirements.e2e.is_empty() || sandbox && e2e_only));

    let tests_run: Vec<String> = executed.iter().map(|r| r.command.clone()).collect();
    let failures: Vec<String> = executed
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.command.clone())
        .collect();

    let mut verified = failures.is_empty();
    let mut reason = None;

    if missing_due_to_plan
        || strict_with_no_runnable_tests
        || (feature.tdd_mode == TddMode::Strict && tests_run.is_empty())
    {
        verified = false;
        reason = Some("missing tests".to_string());
    } else if !failures.is_empty() {
        verified = false;
        let joined = failures.join(",");
        reason = Some(format!("failed: {joined}"));
    }

    let mut result = VerificationResult::new(verified, tests_run, summary);
    if let Some(reason) = reason {
        result = result.with_reason(reason);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backlog::Feature;
    use crate::backlog::TestRequirements;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_git_diff_to_paths() {
        let output = "code-auto-drive-core/src/lib.rs\nREADME.md\n";
        let paths = parse_git_diff_output(output);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("code-auto-drive-core/src/lib.rs"));
    }

    #[test]
    fn strict_mode_marks_missing_tests() {
        let feature = Feature {
            id: "F-1".to_string(),
            description: "demo".to_string(),
            tdd_mode: TddMode::Strict,
            ..Default::default()
        };
        let plan = generate_quick_plan(&[feature]);
        assert_eq!(plan.missing, vec!["F-1".to_string()]);
    }

    #[test]
    fn generates_quick_and_full_commands() {
        let feature = Feature {
            id: "F-2".to_string(),
            description: "demo".to_string(),
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec!["cargo test --all-features".to_string()],
            },
            ..Default::default()
        };
        let plan = generate_quick_plan(&[feature]);
        assert!(
            plan.quick
                .iter()
                .any(|t| t.contains("code-auto-drive-core"))
        );
        assert!(plan.full.iter().any(|t| t.contains("--all-features")));
    }

    #[test]
    fn sandbox_skips_e2e_and_marks_missing_for_strict() {
        let feature = Feature {
            id: "F-3".to_string(),
            description: "needs e2e".to_string(),
            tdd_mode: TddMode::Strict,
            test_requirements: TestRequirements {
                unit: vec![],
                e2e: vec!["cargo test --package ui-e2e".to_string()],
            },
            ..Default::default()
        };
        let plan = super::generate_quick_plan_with_sandbox(std::slice::from_ref(&feature), true);
        assert!(plan.full.iter().all(|cmd| !cmd.contains("ui-e2e")));
        assert!(plan.missing.contains(&feature.id));
    }

    #[test]
    fn sandbox_does_not_add_default_full_plan() {
        let feature = Feature {
            id: "F-7".to_string(),
            description: "no e2e".to_string(),
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };

        let plan = super::generate_quick_plan_with_sandbox(std::slice::from_ref(&feature), true);
        assert!(plan.full.is_empty());
    }

    #[test]
    fn non_sandbox_adds_default_full_plan() {
        let feature = Feature {
            id: "F-8".to_string(),
            description: "no e2e".to_string(),
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };

        let plan = super::generate_quick_plan_with_sandbox(std::slice::from_ref(&feature), false);
        assert_eq!(plan.full, vec!["cargo test --all-features".to_string()]);
    }

    #[test]
    fn strict_mode_without_runs_fails_verification() {
        let feature = Feature {
            id: "F-4".to_string(),
            description: "strict feature".to_string(),
            tdd_mode: TddMode::Strict,
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };
        let plan = generate_quick_plan(std::slice::from_ref(&feature));
        let result = verification_result_for_feature(&feature, &plan, &[], "no tests");
        assert!(!result.verified);
        assert_eq!(result.reason.as_deref(), Some("missing tests"));
    }

    #[test]
    fn verification_records_failures_and_outputs_reason() {
        let feature = Feature {
            id: "F-5".to_string(),
            description: "with tests".to_string(),
            tdd_mode: TddMode::Relaxed,
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };
        let plan = generate_quick_plan(std::slice::from_ref(&feature));
        let executed = vec![
            TestCommandResult::success("cargo test -p code-auto-drive-core"),
            TestCommandResult::failure("cargo test ui", "flaky"),
        ];
        let result = verification_result_for_feature(&feature, &plan, &executed, "tests executed");
        assert!(!result.verified);
        assert_eq!(result.reason.as_deref(), Some("failed: cargo test ui"));
        assert_eq!(
            result.tests_run,
            vec![
                "cargo test -p code-auto-drive-core".to_string(),
                "cargo test ui".to_string()
            ]
        );
    }

    #[test]
    fn verification_succeeds_when_all_pass() {
        let feature = Feature {
            id: "F-6".to_string(),
            description: "passing".to_string(),
            tdd_mode: TddMode::Relaxed,
            test_requirements: TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };
        let plan = generate_quick_plan(std::slice::from_ref(&feature));
        let executed = vec![TestCommandResult::success(
            "cargo test -p code-auto-drive-core",
        )];
        let result = verification_result_for_feature(&feature, &plan, &executed, "all good");
        assert!(result.verified);
        assert_eq!(result.reason, None);
        assert_eq!(
            result.tests_run,
            vec!["cargo test -p code-auto-drive-core".to_string()]
        );
    }
}
