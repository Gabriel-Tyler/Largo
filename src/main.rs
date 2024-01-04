use anyhow::Result;

use largo;

fn main() -> Result<()> {
    largo::run_repl()?;
    Ok(())
}

