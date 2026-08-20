use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "android" {
        let ndk = env::var("ANDROID_NDK_HOME").or_else(|_| env::var("NDK_HOME")).or_else(|_| {
            let home = env::var("HOME").unwrap_or_default();
            let p = PathBuf::from(home).join("Library/Android/sdk/ndk");
            if p.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&p) {
                    if let Some(Ok(e)) = entries.next() {
                        return Ok(e.path().to_string_lossy().to_string());
                    }
                }
            }
            Err(env::VarError::NotPresent)
        });

        if let Ok(ndk_path) = ndk {
            let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "aarch64".to_string());
            let triple = match arch.as_str() {
                "aarch64" => "aarch64-linux-android",
                "arm" => "arm-linux-androideabi",
                "x86_64" => "x86_64-linux-android",
                _ => "i686-linux-android",
            };

            let prebuilt = PathBuf::from(&ndk_path).join("toolchains/llvm/prebuilt");
            if let Ok(entries) = std::fs::read_dir(&prebuilt) {
                for entry in entries.flatten() {
                    let sysroot_lib = entry.path().join("sysroot/usr/lib").join(triple);
                    if sysroot_lib.exists() {
                        println!("cargo:rustc-link-search=native={}", sysroot_lib.display());
                        break;
                    }
                }
            }
        }
        println!("cargo:rustc-link-lib=static=c++_static");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
}
