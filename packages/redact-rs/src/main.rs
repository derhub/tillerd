//! `redact` CLI: read text from stdin, redact, write to stdout.

use std::io::{Read, Write};

fn main() -> std::io::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let out = redact::redact(&input);
    std::io::stdout().write_all(out.as_bytes())?;
    Ok(())
}
