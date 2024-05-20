use std::env;
use os_info::Type;

fn main() {
    let info = os_info::get();
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    if info.os_type() == Type::Macos && env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "macos" {
      // Remove hardcoded path to avoid need to use install_name_tool.
      // Drop file into a well-known path such as /usr/local/lib and it can be automatically discovered
      println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,libpact_ffi.dylib");
    }
}
