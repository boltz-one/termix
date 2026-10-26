//! `main` — Binary entry point. Load `app::app::run()` để build app +
//! mở cửa sổ gốc với `TerminalPage`.

fn main() -> anyhow::Result<()> {
    app::app::run()
}
