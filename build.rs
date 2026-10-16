use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/windows.rc");
    println!("cargo:rerun-if-changed=assets/luma.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    if target.ends_with("gnu") {
        let object = out.join("luma-res.o");
        let compiler = if env::var("HOST").unwrap_or_default().contains("windows") {
            "windres"
        } else {
            "x86_64-w64-mingw32-windres"
        };
        let status = Command::new(compiler)
            .args(["-I", "assets", "assets/windows.rc", "-O", "coff", "-o"])
            .arg(&object)
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            println!("cargo:rustc-link-arg={}", object.display())
        } else {
            println!(
                "cargo:warning=Resource compiler unavailable; runtime tray icon is still available"
            )
        }
    } else {
        let object = out.join("luma.res");
        let status = Command::new("rc.exe")
            .arg("/nologo")
            .arg(format!("/fo{}", object.display()))
            .args(["/I", "assets", "assets/windows.rc"])
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            println!("cargo:rustc-link-arg={}", object.display())
        } else {
            println!("cargo:warning=rc.exe not found; resource embedding skipped")
        }
    }
}
