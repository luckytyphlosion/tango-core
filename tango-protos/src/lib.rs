pub mod ipc {
    include!(concat!(env!("OUT_DIR"), "/tango.core.ipc.rs"));
}

pub mod netplay {
    include!(concat!(env!("OUT_DIR"), "/tango.core.netplay.rs"));
}

pub mod matchmaking {
    include!(concat!(env!("OUT_DIR"), "/tango.matchmaking.rs"));
}
