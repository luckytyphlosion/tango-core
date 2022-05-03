use crate::{facade, fastforwarder};

mod bn6;

lazy_static! {
    pub static ref HOOKS: std::collections::HashMap<String, &'static Box<dyn Hooks + Send + Sync>> = {
        let mut hooks =
            std::collections::HashMap::<String, &'static Box<dyn Hooks + Send + Sync>>::new();
        hooks.insert("MEGAMAN6_FXX".to_string(), &bn6::MEGAMAN6_FXX);
        hooks.insert("MEGAMAN6_GXX".to_string(), &bn6::MEGAMAN6_GXX);
        hooks.insert("ROCKEXE6_RXX".to_string(), &bn6::ROCKEXE6_RXX);
        hooks.insert("ROCKEXE6_GXX".to_string(), &bn6::ROCKEXE6_GXX);
        hooks
    };
}

pub trait Hooks {
    fn fastforwarder_traps(
        &self,
        ff_state: fastforwarder::State,
    ) -> Vec<(u32, Box<dyn FnMut(mgba::core::CoreMutRef)>)>;

    fn shadow_traps(&self) -> Vec<(u32, Box<dyn FnMut(mgba::core::CoreMutRef)>)>;

    fn primary_traps(
        &self,
        handle: tokio::runtime::Handle,
        facade: facade::Facade,
    ) -> Vec<(u32, Box<dyn FnMut(mgba::core::CoreMutRef)>)>;

    fn audio_traps(
        &self,
        facade: facade::AudioFacade,
    ) -> Vec<(u32, Box<dyn FnMut(mgba::core::CoreMutRef)>)>;

    fn prepare_for_fastforward(&self, core: mgba::core::CoreMutRef);

    fn current_tick(&self, core: mgba::core::CoreMutRef) -> u32;
}
