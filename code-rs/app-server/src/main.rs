use code_app_server::run_main;
use code_arg0::arg0_dispatch_or_else;
use code_common::CliConfigOverrides;

fn main() -> anyhow::Result<()> {
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
