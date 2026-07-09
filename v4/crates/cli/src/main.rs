//! OmniMon CLI binary entrypoint.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    if let Err(code) = cli::run() {
        std::process::exit(code);
    }
}
