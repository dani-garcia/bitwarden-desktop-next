use std::{env::args_os, process::exit};

fn main() {
    let mut args = args_os().peekable();

    if args.len() != 1 {
        eprintln!("This command must be run as `cargo run --bin packager`");
        exit(1);
    }
    args.next(); // Skip the binary name

    cargo_packager::cli::run(args, None)
}
