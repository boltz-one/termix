//! `core` — port skeleton phase 1.
//!
//! Native Rust + GPUI port of Windows Terminal's text-buffer / VT-parser
//! core. Ports:
//! - `windows/src/buffer/out/{textBuffer,Row,TextAttribute,LineRendition,cursor}.h*`
//! - `windows/src/terminal/parser/OutputStateMachineEngine.hpp` (adapter on `vte`)
//! - `windows/src/terminal/adapter/ITermDispatch.hpp` (`ITermDispatch` trait, 123 methods)
//! - `windows/src/types/sgrStack.{hpp,cpp}` (`MSFT:15996906`)
//!
//! Phase 1 skeleton: full API surface + `todo!()` bodies, đủ để renderer /
//! control compile against `core::TextBuffer` / `Row` /
//! `TextAttribute` / `Cursor` / `Viewport`. Implementation lands phase 2.
//!
//! TEMPORARY (P01 → P03 removes): `#![allow(non_snake_case)]` để clippy -D warnings
//! clean ở wave-0 barrier trên các PascalCase fn names còn sót (P03 rename).
//!
//! Doc comments Tiếng Việt với em-dash `—` ở đầu continuation line trigger
//! `clippy::doc_lazy_continuation` (rust 1.99 escalation). Văn phong này an
//! toàn (không phải markdown quote thực) → allow crate-wide.

#![allow(non_snake_case)]
#![allow(clippy::doc_lazy_continuation)]

pub mod attrs;
pub mod buffer;
pub mod color_table;
pub mod cursor;
pub mod dispatch;
pub mod parser;
pub mod screen;
pub mod sgr_stack;
pub mod terminal;
pub mod terminal_input;

pub use attrs::{LineRendition, TextAttribute};
pub use buffer::{CellIter, Row, TextBuffer};
pub use cursor::{Cursor, CursorShape};
pub use dispatch::interact_impl::{HostContext, TerminalInteractDispatch};
pub use dispatch::{ITermDispatch, InteractDispatch};
pub use screen::Viewport;
pub use sgr_stack::{SgrSaveOptions, SgrStack};
pub use terminal::Terminal;
pub use terminal_input::InputEvent;
