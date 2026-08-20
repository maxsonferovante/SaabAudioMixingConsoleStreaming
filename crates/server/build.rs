fn main() {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        println!("cargo:rustc-link-search=native=/usr/lib/swift");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

        if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                let dev_path = path_str.trim();
                let swift_lib_dir = format!("{}/usr/lib/swift/macosx", dev_path);
                println!("cargo:rustc-link-search=native={}", swift_lib_dir);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", swift_lib_dir);

                let toolchain_swift_dir = format!(
                    "{}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx",
                    dev_path
                );
                println!("cargo:rustc-link-search=native={}", toolchain_swift_dir);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", toolchain_swift_dir);
            }
        }

        println!("cargo:rustc-link-search=native=/Library/Developer/CommandLineTools/usr/lib/swift/macosx");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/usr/lib/swift/macosx");
    }
}
