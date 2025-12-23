use code_app_server::run_main;
use code_arg0::arg0_dispatch_or_else;
use code_common::CliConfigOverrides;

const CODE_SECURE_MODE_ENV_VAR: &str = "CODE_SECURE_MODE";
const CODEX_SECURE_MODE_ENV_VAR: &str = "CODEX_SECURE_MODE";

fn main() -> anyhow::Result<()> {
    apply_secure_mode();

    // Install a panic hook that prints the panic info to stderr with backtrace
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // We use eprintln! directly to ensure it goes to stderr even if tracing isn't set up yet
        eprintln!("Panic occurred: {:?}", info);
        if let Ok(var) = std::env::var("RUST_BACKTRACE") {
            if var != "0" {
                let backtrace = std::backtrace::Backtrace::capture();
                eprintln!("Backtrace:\n{}", backtrace);
            }
        }
        hook(info);
    }));

    arg0_dispatch_or_else(|code_linux_sandbox_exe| async move {
        run_main(code_linux_sandbox_exe, CliConfigOverrides::default()).await?;
        Ok(())
    })
}

fn apply_secure_mode() {
    let secure_mode = match std::env::var(CODE_SECURE_MODE_ENV_VAR) {
        Ok(value) => value,
        Err(_) => match std::env::var(CODEX_SECURE_MODE_ENV_VAR) {
            Ok(value) => {
                eprintln!(
                    "Deprecated env var {CODEX_SECURE_MODE_ENV_VAR} is set. Use {CODE_SECURE_MODE_ENV_VAR} instead."
                );
                value
            }
            Err(_) => return,
        },
    };

    if secure_mode == "1" {
        code_process_hardening::pre_main_hardening();
    }

    // Always clear this env var so child processes don't inherit it.
    unsafe {
        std::env::remove_var(CODE_SECURE_MODE_ENV_VAR);
        std::env::remove_var(CODEX_SECURE_MODE_ENV_VAR);
    }
}
