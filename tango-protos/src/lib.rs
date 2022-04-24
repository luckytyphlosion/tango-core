pub mod ipc {
    include!(concat!(env!("OUT_DIR"), "/tango.core.ipc.rs"));
}

pub mod netplay {
    include!(concat!(env!("OUT_DIR"), "/tango.core.netplay.rs"));
}
