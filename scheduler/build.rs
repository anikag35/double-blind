fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Neither tonic-prost-build nor Cargo's own default watches a file
    // outside the crate directory - without this, editing the proto with
    // no corresponding Rust-side edit silently keeps using stale generated code.
    println!("cargo:rerun-if-changed=../proto/blind.proto");
    tonic_prost_build::compile_protos("../proto/blind.proto")?;
    Ok(())
}
