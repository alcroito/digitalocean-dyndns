use anyhow::Result;
use vergen_gitcl::{Build, Cargo, Emitter, Gitcl, Rustc, Sysinfo};

fn main() -> Result<()> {
    Emitter::default()
        .add_instructions(&Build::all_build())?
        .add_instructions(&Cargo::all_cargo())?
        .add_instructions(&Gitcl::all_git())?
        .add_instructions(&Rustc::all_rustc())?
        .add_instructions(&Sysinfo::all_sysinfo())?
        .emit()?;
    Ok(())
}
