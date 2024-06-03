fn main() {
    cc::Build::new()
        .file("src/ffi/passwd.c")
        .compile("vish");
    println!("cargo:rerun-if-changed=src/ffi/passwd.c");
}
