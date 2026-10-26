//! `app` — port của `windows/src/cascadia/TerminalApp/`.
//!
//! Phase 2: wire Продcrate root với `Terminal Page` + `Tab` + `Pane` +
//! `settings_loader` + `downstream` (Lane B concrete API aliases).
//! `app::run` dựng `Application`, mở cửa sổ gốc với `TerminalPage` làm root.

pub mod app;
pub mod downstream;
#[cfg(not(feature = "stub-downstream"))]
pub mod null_render_engine;
pub mod page;
pub mod pane;
pub mod pane_tree;
pub mod settings_loader;
pub mod tab;
pub mod ui;

pub use app::run;
pub use page::TerminalPage;
