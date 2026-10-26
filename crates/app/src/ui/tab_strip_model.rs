//! `TabStripModel` — state thuần của tab row (chọn / đóng / kéo đổi chỗ).
//! Port hành vi `TabView` + `TerminalPage::_ReorderTab` / `_RemoveTab`
//! (`TabManagement.cpp`). Không phụ thuộc GPUI nên test được trực tiếp.

/// Chỉ báo trạng thái trên tab header (`TabHeaderControl.xaml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TabStatus {
    pub bell: bool,
    pub zoomed: bool,
    pub read_only: bool,
    pub connection_closed: bool,
    pub broadcast: bool,
}

impl TabStatus {
    /// Glyph (Segoe MDL2) theo thứ tự hiển thị của `HeaderStackPanel`.
    pub fn glyphs(&self) -> Vec<char> {
        let mut g = Vec::new();
        if self.bell {
            g.push('\u{EA8F}');
        }
        if self.zoomed {
            g.push('\u{E8A3}');
        }
        if self.read_only {
            g.push('\u{E72E}');
        }
        if self.connection_closed {
            g.push('\u{EA39}');
        }
        if self.broadcast {
            g.push('\u{EC05}');
        }
        g
    }
}

/// 1 tab trên strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabItem {
    pub title: String,
    pub status: TabStatus,
}

impl TabItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), status: TabStatus::default() }
    }

    /// Title hiển thị; rỗng → "Terminal" (control chưa raise `TitleChanged`).
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            "Terminal"
        } else {
            &self.title
        }
    }
}

/// Danh sách tab + tab đang chọn.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabStripModel {
    items: Vec<TabItem>,
    selected: Option<usize>,
}

impl TabStripModel {
    pub fn new(items: Vec<TabItem>, selected: Option<usize>) -> Self {
        let selected = selected.filter(|&i| i < items.len());
        Self { items, selected }
    }

    pub fn items(&self) -> &[TabItem] {
        &self.items
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Thêm tab cuối và chọn nó (`_OpenNewTab` luôn switch sang tab mới).
    pub fn push(&mut self, item: TabItem) {
        self.items.push(item);
        self.selected = Some(self.items.len() - 1);
    }

    /// Chọn tab; index ngoài range bị bỏ qua.
    pub fn select(&mut self, index: usize) -> bool {
        if index < self.items.len() {
            self.selected = Some(index);
            true
        } else {
            false
        }
    }

    /// Đóng tab. Đóng tab đang chọn → chọn tab cuối còn lại (như
    /// `TerminalPage::handle_close_tab_action`); đóng tab trước → dịch index.
    pub fn close(&mut self, index: usize) -> Option<TabItem> {
        if index >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(index);
        self.selected = match self.selected {
            _ if self.items.is_empty() => None,
            Some(s) if s == index => Some(self.items.len() - 1),
            Some(s) if s > index => Some(s - 1),
            other => other,
        };
        Some(removed)
    }

    /// Kéo tab `from` tới vị trí `to` (index sau khi di chuyển); selection đi
    /// theo item được chọn (`TabView.TabItemsChanged` → `_ReorderTab`).
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        let len = self.items.len();
        if from >= len || to >= len {
            return false;
        }
        if from == to {
            return true;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        self.selected = self.selected.map(|s| {
            if s == from {
                to
            } else if from < s && s <= to {
                s - 1
            } else if to <= s && s < from {
                s + 1
            } else {
                s
            }
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(n: usize, sel: usize) -> TabStripModel {
        let items = (0..n).map(|i| TabItem::new(format!("t{i}"))).collect();
        TabStripModel::new(items, Some(sel))
    }

    fn titles(m: &TabStripModel) -> Vec<&str> {
        m.items().iter().map(|i| i.title.as_str()).collect()
    }

    #[test]
    fn push_selects_new_tab() {
        let mut m = model(2, 0);
        m.push(TabItem::new("x"));
        assert_eq!(m.selected(), Some(2));
    }

    #[test]
    fn close_active_falls_back_to_last() {
        let mut m = model(3, 0);
        assert!(m.close(0).is_some());
        assert_eq!(m.selected(), Some(1));
        assert_eq!(titles(&m), ["t1", "t2"]);
    }

    #[test]
    fn close_before_active_shifts_down() {
        let mut m = model(3, 2);
        m.close(0);
        assert_eq!(m.selected(), Some(1));
    }

    #[test]
    fn close_last_empties_selection_and_oob_ignored() {
        let mut m = model(1, 0);
        assert!(m.close(5).is_none());
        m.close(0);
        assert_eq!(m.selected(), None);
    }

    #[test]
    fn drag_0_to_2_reorders_and_selection_follows() {
        let mut m = model(3, 0);
        assert!(m.move_tab(0, 2));
        assert_eq!(titles(&m), ["t1", "t2", "t0"]);
        assert_eq!(m.selected(), Some(2));
    }

    #[test]
    fn move_adjusts_unmoved_selection() {
        let mut m = model(4, 2);
        m.move_tab(0, 3); // t0 vượt qua selected → selected dịch xuống
        assert_eq!(m.selected(), Some(1));
        let mut m = model(4, 1);
        m.move_tab(3, 0); // t3 chèn trước selected → selected dịch lên
        assert_eq!(m.selected(), Some(2));
        assert!(!m.move_tab(0, 9));
    }

    #[test]
    fn status_glyph_order() {
        let s = TabStatus { bell: true, read_only: true, ..Default::default() };
        assert_eq!(s.glyphs(), vec!['\u{EA8F}', '\u{E72E}']);
        assert_eq!(TabItem::new("").display_title(), "Terminal");
    }
}
