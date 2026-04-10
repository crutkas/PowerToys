fn main() {
    // Compile the C++ adapter that wraps Rust extern "C" functions
    // in a PowertoyModuleIface vtable with powertoy_create() export.
    cc::Build::new()
        .cpp(true)
        .file("cpp/adapter.cpp")
        .include("cpp")
        .std("c++20")
        .compile("adapter");

    // Ensure powertoy_create is exported from the final DLL.
    // __declspec(dllexport) is lost when cc compiles to a static .lib first,
    // so we tell the MSVC linker directly.
    println!("cargo::rustc-cdylib-link-arg=/EXPORT:powertoy_create");

    println!("cargo::rerun-if-changed=cpp/adapter.cpp");
    println!("cargo::rerun-if-changed=cpp/powertoy_module_interface.h");
}
