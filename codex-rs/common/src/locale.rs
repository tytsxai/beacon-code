#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    #[must_use]
    pub fn detect() -> Self {
        let codex_lang = std::env::var("CODEX_LANG").ok();
        let lang = std::env::var("LANG").ok();
        Self::from_env(codex_lang, lang)
    }

    fn from_env(codex_lang: Option<String>, lang: Option<String>) -> Self {
        if let Some(lang) = codex_lang {
            if Self::is_zh(&lang) {
                return Self::Chinese;
            }
        }

        if let Some(lang) = lang {
            if Self::is_zh(&lang) {
                return Self::Chinese;
            }
        }

        Self::English
    }

    fn is_zh(lang: &str) -> bool {
        let lower = lang.to_ascii_lowercase();
        lower.starts_with("zh")
    }

    #[must_use]
    pub fn is_chinese(self) -> bool {
        matches!(self, Self::Chinese)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::Mutex;

    // Environment manipulation in tests is racy; serialize them.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    #[test]
    fn detect_prefers_codex_lang() {
        let _guard = ENV_GUARD.lock().expect("lock env");
        let original_codex = std::env::var("CODEX_LANG").ok();
        let original_lang = std::env::var("LANG").ok();

        std::env::set_var("CODEX_LANG", "zh_CN.UTF-8");
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(Language::detect(), Language::Chinese);

        if let Some(val) = original_codex {
            std::env::set_var("CODEX_LANG", val);
        } else {
            std::env::remove_var("CODEX_LANG");
        }

        if let Some(val) = original_lang {
            std::env::set_var("LANG", val);
        } else {
            std::env::remove_var("LANG");
        }
    }

    #[test]
    fn detect_falls_back_to_lang() {
        let _guard = ENV_GUARD.lock().expect("lock env");
        let original_codex = std::env::var("CODEX_LANG").ok();
        let original_lang = std::env::var("LANG").ok();

        std::env::remove_var("CODEX_LANG");
        std::env::set_var("LANG", "zh_TW.UTF-8");
        assert_eq!(Language::detect(), Language::Chinese);

        if let Some(val) = original_codex {
            std::env::set_var("CODEX_LANG", val);
        } else {
            std::env::remove_var("CODEX_LANG");
        }

        if let Some(val) = original_lang {
            std::env::set_var("LANG", val);
        } else {
            std::env::remove_var("LANG");
        }
    }

    #[test]
    fn detect_defaults_to_english() {
        let _guard = ENV_GUARD.lock().expect("lock env");
        let original_codex = std::env::var("CODEX_LANG").ok();
        let original_lang = std::env::var("LANG").ok();

        std::env::remove_var("CODEX_LANG");
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(Language::detect(), Language::English);

        if let Some(val) = original_codex {
            std::env::set_var("CODEX_LANG", val);
        } else {
            std::env::remove_var("CODEX_LANG");
        }

        if let Some(val) = original_lang {
            std::env::set_var("LANG", val);
        } else {
            std::env::remove_var("LANG");
        }
    }
}
