fn main() {
    println!("cargo:rerun-if-changed=assets/mewtion.rc");
    println!("cargo:rerun-if-changed=assets/mewtion.ico");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("mewtion.res");
    let rc_file = std::path::Path::new("assets/mewtion.rc");

    if !rc_file.exists() {
        return;
    }

    // Attempt to compile the .rc file using standard Windows SDK rc.exe
    let status = std::process::Command::new("rc.exe")
        .args([
            "/nologo",
            "/fo",
            res_path.to_str().unwrap(),
            rc_file.to_str().unwrap(),
        ])
        .status();

    if let Ok(s) = status {
        if s.success() && res_path.exists() {
            println!("cargo:rustc-link-arg={}", res_path.display());
        }
    }
}
