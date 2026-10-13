use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=TARGET");
    let rustc = env::var_os("RUSTC").expect("Cargo supplies RUSTC");
    let output = Command::new(rustc).arg("-vV").output().expect("rustc -vV");
    assert!(output.status.success(), "rustc -vV failed");
    let identity = String::from_utf8(output.stdout).expect("rustc identity is UTF-8");
    assert!(identity.starts_with("rustc "), "invalid rustc identity");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"));
    fs::write(out.join("rustc-identity.txt"), identity).expect("write compiler identity");
    fs::write(out.join("target-triple.txt"), env::var("TARGET").expect("Cargo supplies TARGET"))
        .expect("write target triple");
}
