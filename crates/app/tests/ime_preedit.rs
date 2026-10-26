//! IME composition lifecycle of `TermControl` (preedit shown, then committed).
use gpui::{EntityInputHandler, TestAppContext};
use app::null_render_engine::NullRenderEngine;
use connection::ConptyConnection;
use control::{KeyMapping, TermControl};
use core::{TextAttribute, TextBuffer};
use settings::Profile;

#[gpui::test]
fn preedit_is_marked_then_cleared_on_commit(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, cx| {
        TermControl::new(
            Box::new(ConptyConnection::new()),
            Box::new(NullRenderEngine),
            TextBuffer::new(80, 25, TextAttribute::default()),
            Profile::default_powershell(),
            KeyMapping::default(),
            cx,
        )
    });
    window
        .update(cx, |t, window, cx| {
            assert_eq!(t.marked_text_range(window, cx), None);
            t.replace_and_mark_text_in_range(None, "\u{3105}\u{3106}", None, window, cx);
            assert_eq!(t.marked_text_range(window, cx), Some(0..2));
            assert_eq!(t.selected_text_range(false, window, cx).unwrap().range, 2..2);
            // A longer composition replaces the previous one.
            t.replace_and_mark_text_in_range(None, "\u{4F60}\u{597D}\u{5417}", None, window, cx);
            assert_eq!(t.marked_text_range(window, cx), Some(0..3));
            // Committing clears the composition.
            t.replace_text_in_range(None, "\u{4F60}\u{597D}", window, cx);
            assert_eq!(t.marked_text_range(window, cx), None);
            // Cancelling a composition also clears it.
            t.replace_and_mark_text_in_range(None, "x", None, window, cx);
            t.unmark_text(window, cx);
            assert_eq!(t.marked_text_range(window, cx), None);
        })
        .unwrap();
}
