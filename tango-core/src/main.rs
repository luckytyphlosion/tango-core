#![windows_subsystem = "windows"]

use clap::StructOpt;

#[derive(clap::Parser)]
struct Cli {
    #[clap(long)]
    pub window_title: String,

    #[clap(parse(from_os_str))]
    pub rom_path: std::path::PathBuf,

    #[clap(parse(from_os_str))]
    pub save_path: std::path::PathBuf,

    #[clap(long)]
    pub keymapping: String,

    #[clap(long)]
    pub match_settings: Option<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

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
        serde_json::from_str(&args.keymapping)?,
        args.rom_path.into(),
        args.save_path.into(),
        args.match_settings
            .map(|raw| serde_json::from_str(&raw))
            .map_or(Ok(None), |v| v.map(Some))?,
    )?;
    g.run()?;
    Ok(())
}
