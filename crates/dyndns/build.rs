use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .git_describe(true, true)
        .all_rustc()
        .all_sysinfo()
        .emit()?;
    Ok(())
}
