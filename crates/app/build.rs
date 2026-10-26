//! Embed the Windows Terminal icon as resource id 1 (what GPUI's window class
//! and the shell use for the taskbar / Alt+Tab / exe icon).

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/terminal.rc");
        println!("cargo:rerun-if-changed=assets/terminal.ico");
        embed_resource::compile_for("assets/terminal.rc", ["termix"], embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
