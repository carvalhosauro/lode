//! Lode CLI entry point. Verbs (`ingest` / `open` / `query` / `validate` / `status`)
//! land with RFC-0016 (Configuration & CLI Model).

use clap::Parser;

/// Lode — local-first log investigation engine.
#[derive(Debug, Parser)]
#[command(name = "lode", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!(
        "lode {} — scaffold. See RFC/ and ROADMAP.md.",
        lode_core::VERSION
    );
}
