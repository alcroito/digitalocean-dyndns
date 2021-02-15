use anyhow::Result;

pub fn setup_early_logger() -> Result<()> {
    log_reroute::init()?;
    let early_logger = create_fern_dispatch(&log::LevelFilter::Trace);
    log_reroute::reroute_boxed(early_logger);
    Ok(())
}

pub fn setup_logger(log_level: &log::LevelFilter) -> Result<()> {
    let main_logger = create_fern_dispatch(log_level);
    log_reroute::reroute_boxed(main_logger);
    Ok(())
}

fn create_fern_dispatch(log_level: &log::LevelFilter) -> Box<dyn log::Log> {
    fern::Dispatch::new()
        .format(format_callback)
        .level(*log_level)
        .chain(std::io::stdout())
        .into_log()
        .1
}

fn format_callback(
    out: fern::FormatCallback,
    message: &core::fmt::Arguments,
    record: &log::Record,
) {
    out.finish(format_args!(
        "{} [{}] {}",
        chrono::Local::now().format("%b %d %H:%M:%S"),
        record.level(),
        message
    ))
}
