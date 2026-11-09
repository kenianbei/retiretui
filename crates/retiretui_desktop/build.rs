//! Embeds the icon in the Windows executable's resources.

fn main() {
    println!("cargo:rerun-if-changed=icon.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("icon.ico")
        .compile()
        .expect("the icon compiles into the executable's resources");
}
