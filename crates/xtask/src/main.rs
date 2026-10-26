//! Entry bin — `cargo xtask <command> [args]`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = xtask::run(args);
    std::process::exit(code);
}
