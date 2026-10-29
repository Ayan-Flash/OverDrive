// Always a GUI app — never show a console window, even on panic.
// The conditional `cfg_attr` pattern is fragile across toolchains, so we set
// it unconditionally. Debug logging goes to the file logger, not stdout.
#![windows_subsystem = "windows"]

// Thin binary entry point. All application logic lives in the library crate
// (`lib.rs::run`) so modules are compiled once (not duplicated between the bin
// and lib) and the app stays mobile-entry-point compatible.
fn main() {
    // Catch any panic so the process exits silently instead of showing
    // a Windows abort dialog or allocating a console for stderr output.
    let result = std::panic::catch_unwind(|| {
        overdrive_lib::run();
    });

    if let Err(_panic) = result {
        // Panic already logged by the custom hook in init_logging.
        // Exit cleanly without the default abort handler.
        std::process::exit(1);
    }
}
