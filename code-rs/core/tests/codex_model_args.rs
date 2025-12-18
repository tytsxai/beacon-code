use code_core::agent_defaults::agent_model_spec;

#[test]
fn codex_specs_have_expected_model_args_and_aliases() {
    let max = agent_model_spec("code-gpt-5.1-codex-max").expect("spec present");
    assert_eq!(max.model_args, ["--model", "gpt-5.1-codex-max"]);

    let by_alias = agent_model_spec("codex").expect("alias present");
    assert_eq!(by_alias.slug, "code-gpt-5.1-codex-max");

    let by_cli = agent_model_spec("coder").expect("cli alias present");
    assert_eq!(by_cli.slug, "code-gpt-5.1-codex-max");

    let mini = agent_model_spec("code-gpt-5.1-codex-mini").expect("spec present");
    assert_eq!(mini.model_args, ["--model", "gpt-5.1-codex-mini"]);

    let cloud = agent_model_spec("cloud").expect("cloud alias present");
    assert_eq!(cloud.slug, "cloud-gpt-5.1-codex-max");
}
