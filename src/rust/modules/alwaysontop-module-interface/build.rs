fn main() {
    cc::Build::new()
        .cpp(true)
        .file("cpp/adapter.cpp")
        .include("cpp")
        .std("c++20")
        .compile("adapter");

    println!("cargo::rustc-cdylib-link-arg=/EXPORT:powertoy_create");
    println!("cargo::rerun-if-changed=cpp/adapter.cpp");
    println!("cargo::rerun-if-changed=cpp/powertoy_module_interface.h");
}
