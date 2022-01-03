use anyhow::Result;
use vergen::{vergen, Config, SemverKind, TimestampKind};

fn main() -> Result<()> {
    let mut config = Config::default();
    // Enable build date generation
    *config.build_mut().kind_mut() = TimestampKind::All;

    // Change the SEMVER output to the lightweight variant
    *config.git_mut().semver_kind_mut() = SemverKind::Lightweight;
    // Add a `-dirty` flag to the SEMVER output
    *config.git_mut().semver_dirty_mut() = Some("-dirty");

    // Generate the instructions
    vergen(config)
}
