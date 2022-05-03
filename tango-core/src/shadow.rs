use crate::hooks;

pub struct Shadow {
    core: mgba::core::Core,
    hooks: &'static Box<dyn hooks::Hooks + Send + Sync>,
    is_offerer: bool,
}

impl Shadow {
    pub fn new(
        rom_path: &std::path::Path,
        hooks: &'static Box<dyn hooks::Hooks + Send + Sync>,
        is_offerer: bool,
    ) -> anyhow::Result<Self> {
        let mut core = {
            let mut core = mgba::core::Core::new_gba("tango")?;
            let rom_vf = mgba::vfile::VFile::open(rom_path, mgba::vfile::flags::O_RDONLY)?;
            core.as_mut().load_rom(rom_vf)?;
            core
        };

        core.set_traps(hooks.shadow_traps());
        core.as_mut().reset();

        Ok(Shadow {
            core,
            hooks,
            is_offerer,
        })
    }
}
