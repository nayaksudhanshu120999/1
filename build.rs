fn main() {
    slint_build::compile("ui/palette.slint").expect("slint compile failed");
    println!("cargo:rerun-if-changed=ui/palette.slint");
}
