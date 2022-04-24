#![windows_subsystem = "windows"]

fn main() -> Result<(), anyhow::Error> {
    let args = tango_core::ipc::Args::parse(
        &std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("missing startup args"))?,
    )?;

    env_logger::Builder::from_default_env()
        .filter(Some("tango_core"), log::LevelFilter::Info)
        .init();

    log::info!(
        "welcome to tango-core v{}-{}!",
        env!("CARGO_PKG_VERSION"),
        git_version::git_version!()
    );

    mgba::log::init();

    let g = tango_core::game::Game::new(
        tango_core::ipc::Client::new_from_stdio(),
        args.window_title,
        args.keymapping.try_into()?,
        args.rom_path.into(),
        args.save_path.into(),
        args.match_settings,
    )?;
    g.run()?;
    Ok(())
}
