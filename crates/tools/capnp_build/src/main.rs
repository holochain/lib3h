extern crate capnpc;

fn main() {
    let mut path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    );
    path.push("..");
    path.push("..");
    path.push("p2p_protocol");
    let path = path.canonicalize().expect("standardize path");

    let mut src_prefix = path.to_path_buf();
    src_prefix.push("protocol");
    let src_prefix = src_prefix.to_string_lossy().to_string();

    let mut p2p_file = path.to_path_buf();
    p2p_file.push("protocol");
    p2p_file.push("p2p.capnp");
    let p2p_file = p2p_file.to_string_lossy().to_string();

    let mut transit_file = path.to_path_buf();
    transit_file.push("protocol");
    transit_file.push("transit_encoding.capnp");
    let transit_file = transit_file.to_string_lossy().to_string();

    let mut output = path.to_path_buf();
    output.push("src");
    let output = output.to_string_lossy().to_string();

    let mut command = std::process::Command::new("capnp");
    command.arg("--version");
    let version =
        String::from_utf8_lossy(&command.output().expect("capnp version").stdout).to_string();
    assert_eq!("Cap'n Proto version 0.7.0\n", version);

    std::env::set_var("OUT_DIR", &output);
    capnpc::CompilerCommand::new()
        .no_standard_import()
        .src_prefix(src_prefix)
        .file(p2p_file)
        .file(transit_file)
        .output_path(output)
        .run()
        .expect("capnp schema compiler");
}
