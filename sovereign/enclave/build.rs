fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .file_descriptor_set_path("src/descriptor.bin")
        .compile_protos(&["../../proto/key_pool.proto"], &["../../"])?;
    Ok(())
}
