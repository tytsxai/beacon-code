use std::borrow::Cow;

#[cfg(target_os = "macos")]
pub(crate) fn macos_brew_formula_for_command(command: &str) -> Cow<'_, str> {
    let trimmed = command.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains(char::is_whitespace) {
        return Cow::Borrowed(trimmed);
    }
    Cow::Borrowed(trimmed)
}
