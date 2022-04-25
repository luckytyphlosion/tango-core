use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/ipc.proto",
            "src/netplay.proto",
            "src/matchmaking.proto",
        ],
        &["src/"],
    )?;
    Ok(())
}
