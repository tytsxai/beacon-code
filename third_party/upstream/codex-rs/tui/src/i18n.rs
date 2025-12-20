use codex_common::locale::Language;
use ratatui::style::Stylize;
use ratatui::text::Line;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strings {
    language: Language,
}

impl Strings {
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    #[must_use]
    pub fn welcome_message(self) -> Line<'static> {
        match self.language {
            Language::Chinese => Line::from(vec![
                "  ".into(),
                "欢迎使用 ".into(),
                "Beacon Code".bold(),
                "，终端里的编程智能体".into(),
            ]),
            Language::English => Line::from(vec![
                "  ".into(),
                "Welcome to ".into(),
                "Beacon Code".bold(),
                ", a command-line coding agent".into(),
            ]),
        }
    }

    #[must_use]
    pub fn trust_heading(self, cwd: &Path) -> Line<'static> {
        let path = cwd.to_string_lossy();
        match self.language {
            Language::Chinese => Line::from(vec![
                "> ".into(),
                "你正在该目录运行 Beacon Code：".bold(),
                path.to_string().into(),
            ]),
            Language::English => Line::from(vec![
                "> ".into(),
                "You are running Beacon Code in ".bold(),
                path.to_string().into(),
            ]),
        }
    }

    #[must_use]
    pub fn trust_guidance(self, is_git_repo: bool) -> &'static str {
        match (self.language, is_git_repo) {
            (Language::Chinese, true) => {
                "该目录受版本控制，可允许 Beacon Code 在此执行而无需逐次审批。"
            }
            (Language::Chinese, false) => "该目录未受版本控制，建议对所有编辑和命令进行审批。",
            (Language::English, true) => {
                "Since this folder is version controlled, you may wish to allow Beacon Code to work in this folder without asking for approval."
            }
            (Language::English, false) => {
                "Since this folder is not version controlled, we recommend requiring approval of all edits and commands."
            }
        }
    }

    #[must_use]
    pub fn trust_options(self, is_git_repo: bool) -> TrustOptions {
        match (self.language, is_git_repo) {
            (Language::Chinese, true) => TrustOptions {
                allow_without_approval: "是，允许 Beacon Code 在此目录工作且不再请求审批",
                require_approval: "否，所有编辑/命令都要审批",
            },
            (Language::Chinese, false) => TrustOptions {
                allow_without_approval: "允许 Beacon Code 在此目录工作且不再请求审批",
                require_approval: "需要审批所有编辑/命令",
            },
            (Language::English, true) => TrustOptions {
                allow_without_approval: "Yes, allow Beacon Code to work in this folder without asking for approval",
                require_approval: "No, ask me to approve edits and commands",
            },
            (Language::English, false) => TrustOptions {
                allow_without_approval: "Allow Beacon Code to work in this folder without asking for approval",
                require_approval: "Require approval of edits and commands",
            },
        }
    }

    #[must_use]
    pub fn trust_continue_hint(self) -> Line<'static> {
        match self.language {
            Language::Chinese => Line::from(vec![
                "按 ".dim(),
                crate::key_hint::plain(crossterm::event::KeyCode::Enter).into(),
                " 继续".dim(),
            ]),
            Language::English => Line::from(vec![
                "Press ".dim(),
                crate::key_hint::plain(crossterm::event::KeyCode::Enter).into(),
                " to continue".dim(),
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustOptions {
    pub allow_without_approval: &'static str,
    pub require_approval: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_common::locale::Language;
    use pretty_assertions::assert_eq;

    #[test]
    fn welcome_message_is_bilingual() {
        let zh = Strings::new(Language::Chinese).welcome_message();
        let en = Strings::new(Language::English).welcome_message();
        assert_eq!(zh.to_string(), "  欢迎使用 Beacon Code，终端里的编程智能体");
        assert_eq!(
            en.to_string(),
            "  Welcome to Beacon Code, a command-line coding agent"
        );
    }

    #[test]
    fn trust_options_switch_by_language_and_repo() {
        let zh_git = Strings::new(Language::Chinese).trust_options(true);
        assert_eq!(
            zh_git.allow_without_approval,
            "是，允许 Beacon Code 在此目录工作且不再请求审批"
        );
        let en_non_git = Strings::new(Language::English).trust_options(false);
        assert_eq!(
            en_non_git.require_approval,
            "Require approval of edits and commands"
        );
    }
}
