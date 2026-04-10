fn main() {
    // Find the PowerToys repo root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo_root = std::path::Path::new(&manifest_dir)
        .ancestors()
        .find(|p| p.join("PowerToys.slnx").exists())
        .map(|p| p.to_path_buf());

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("cpp/adapter.cpp")
        .include("cpp")
        .std("c++20");

    // Use the REAL PowerToys headers when in the source tree
    if let Some(ref root) = repo_root {
        build.define("POWERTOYS_TREE", None);
        build.include(root.join("src\\modules\\interface"));
        build.include(root.join("src\\common\\utils"));
        build.include(root.join("src"));
    }

    build.compile("adapter");

    println!("cargo::rustc-cdylib-link-arg=/EXPORT:powertoy_create");
    println!("cargo::rerun-if-changed=cpp/adapter.cpp");
    println!("cargo::rerun-if-changed=cpp/powertoy_module_interface.h");
}
