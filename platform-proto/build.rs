use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var("PROTOC").is_err() {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

    let root = PathBuf::from("proto");
    let sentinel = root.join("sentinel");
    let nexus = root.join("nexus");
    let atrium = root.join("atrium");

    let mut files: Vec<PathBuf> = std::fs::read_dir(&sentinel)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "proto"))
        .collect();
    files.sort();

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&files, &[sentinel])?;
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[nexus.join("game_server.proto")], &[nexus])?;
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[atrium.join("welcome.proto")], &[atrium])?;
    Ok(())
}
