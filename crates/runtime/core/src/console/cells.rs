//! UTF-8 side table of SCREEN 0 surfaces. The character plane of a text
//! surface holds one code page 437 byte per cell; characters that code page
//! 437 cannot show (Thai, CJK, emoji, combining marks) are kept here as UTF-8
//! so the text renderer can draw them.
//!
//! Cell state: 0 = plain, 1 = UTF-8 cluster, 2 = first half of a wide cluster,
//! 3 = second half of a wide cluster (no content of its own).

use super::utf8::{cp437, push_utf8, unicode_to_cp437};
use std::sync::Mutex;

#[derive(Default, Clone)]
struct Table {
    cells: Vec<Vec<u8>>,
    state: Vec<u8>,
}

/// Indexed by image number. A missing or empty table means "no side table"
/// (graphics surface, or a text surface before `init`).
static TABLES: Mutex<Vec<Table>> = Mutex::new(Vec::new());

fn with<R>(f: impl FnOnce(&mut Vec<Table>) -> R) -> R {
    let mut g = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

fn table(v: &mut [Table], image: i32) -> Option<&mut Table> {
    if image < 0 {
        return None;
    }
    v.get_mut(image as usize)
}

/// `qb_init_text_utf8_cells`: a fresh, empty table of `cells` entries
/// (`width*height` of a text surface).
pub fn init(image: i32, cells: usize) {
    if image < 0 {
        return;
    }
    with(|v| {
        if v.len() <= image as usize {
            v.resize(image as usize + 1, Table::default());
        }
        v[image as usize] = Table { cells: vec![Vec::new(); cells], state: vec![0; cells] };
    })
}

/// `qb_release_text_utf8_cells`.
pub fn release(image: i32) {
    with(|v| {
        if let Some(t) = table(v, image) {
            *t = Table::default();
        }
    })
}

/// `_COPYIMAGE` of a text surface copies its side table.
pub fn copy(src: i32, dst: i32) {
    with(|v| {
        let t = match table(v, src) {
            Some(t) => t.clone(),
            None => return,
        };
        if dst < 0 {
            return;
        }
        if v.len() <= dst as usize {
            v.resize(dst as usize + 1, Table::default());
        }
        v[dst as usize] = t;
    })
}

/// `qb_clear_text_utf8_cells_range`.
pub fn clear_range(image: i32, first: i32, count: i32) {
    with(|v| {
        let Some(t) = table(v, image) else { return };
        if t.cells.is_empty() || count <= 0 {
            return;
        }
        // i64: `first + count` cannot overflow.
        let len = t.cells.len() as i64;
        let first = i64::from(first).max(0);
        let end = (first + i64::from(count)).min(len);
        for i in first..end {
            t.cells[i as usize].clear();
            t.state[i as usize] = 0;
        }
    })
}

/// Scroll of the text window: rows `top+1..=bottom` move up one row and the
/// bottom row is cleared (`newline`).
pub fn scroll_up(image: i32, width: i32, top_row: i32, bottom_row: i32) {
    with(|v| {
        let Some(t) = table(v, image) else { return };
        if t.cells.is_empty() {
            return;
        }
        // Rows outside the table (VIEW PRINT ranges, a WIDTH change before
        // the table was resized) are skipped instead of indexing out of range.
        let len = t.cells.len() as i64;
        let (width, top, bottom) = (i64::from(width), i64::from(top_row), i64::from(bottom_row));
        if width <= 0 || top < 1 || bottom < top {
            return;
        }
        let dst = (top - 1) * width;
        let src = top * width;
        let moved = (bottom - top) * width;
        for c in 0..moved {
            let (d, s) = (dst + c, src + c);
            if s >= len {
                break;
            }
            t.cells[d as usize] = t.cells[s as usize].clone();
            t.state[d as usize] = t.state[s as usize];
        }
        let first = ((bottom - 1) * width).max(0);
        let end = (first + width).min(len);
        for i in first..end {
            t.cells[i as usize].clear();
            t.state[i as usize] = 0;
        }
    })
}

fn normalize(t: &mut Table, i: usize) {
    if t.state[i] == 3 && i > 0 && t.state[i - 1] == 2 {
        t.cells[i - 1].clear();
        t.state[i - 1] = 0;
    }
    if t.state[i] == 2 && i + 1 < t.cells.len() && t.state[i + 1] == 3 {
        t.cells[i + 1].clear();
        t.state[i + 1] = 0;
    }
    t.cells[i].clear();
    t.state[i] = 0;
}

/// `qb_store_text_cell_utf8`: records the code point written to `cell`.
pub fn store(image: i32, cell: i32, character: u32, span: i32) {
    with(|v| {
        let Some(t) = table(v, image) else { return };
        if cell < 0 || cell as usize >= t.cells.len() {
            return;
        }
        let i = cell as usize;
        normalize(t, i);
        if span > 1 && i + 1 < t.cells.len() {
            normalize(t, i + 1);
        }
        if character <= 0x7F || unicode_to_cp437(character) != 0 {
            t.state[i] = 0;
            return;
        }
        push_utf8(&mut t.cells[i], character);
        t.state[i] = if span > 1 { 2 } else { 1 };
        if span > 1 && i + 1 < t.cells.len() {
            t.cells[i + 1].clear();
            t.state[i + 1] = 3;
        }
    })
}

/// `qb_append_text_cell_utf8`: attaches a zero-width code point to the
/// cluster that `cell` belongs to. `raw` is the cell's byte in the character
/// plane (used when the cell has no UTF-8 content yet). Returns false when
/// there is no side table or the cell is out of range.
pub fn append(image: i32, cell: i32, character: u32, raw: impl FnOnce(usize) -> u8) -> bool {
    with(|v| {
        let Some(t) = table(v, image) else { return false };
        let mut cell = cell;
        if cell >= 0 && (cell as usize) < t.state.len() && t.state[cell as usize] == 3 && cell > 0 {
            cell -= 1;
        }
        if cell < 0 || cell as usize >= t.cells.len() {
            return false;
        }
        let i = cell as usize;
        if t.cells[i].is_empty() {
            let r = raw(i);
            if r != 32 {
                push_utf8(&mut t.cells[i], cp437(r));
            }
            t.state[i] = 1;
        }
        push_utf8(&mut t.cells[i], character);
        true
    })
}

/// True when the side table of `image` exists (not empty).
pub fn present(image: i32) -> bool {
    with(|v| table(v, image).is_some_and(|t| !t.cells.is_empty()))
}

/// `qb_text_page_has_utf8_content`.
pub fn has_content(image: i32) -> bool {
    with(|v| table(v, image).is_some_and(|t| t.cells.iter().any(|c| !c.is_empty())))
}

/// `qb_text_cell_span`: 2 (wide), 0 (second half of a wide cell) or 1.
pub fn span(image: i32, cell: i32) -> i32 {
    with(|v| {
        let Some(t) = table(v, image) else { return 1 };
        match t.state.get(cell.max(-1) as usize) {
            Some(2) if cell >= 0 => 2,
            Some(3) if cell >= 0 => 0,
            _ => 1,
        }
    })
}

/// UTF-8 content of a cell (empty if none).
pub fn content(image: i32, cell: i32) -> Vec<u8> {
    with(|v| {
        table(v, image)
            .and_then(|t| if cell < 0 { None } else { t.cells.get(cell as usize) })
            .cloned()
            .unwrap_or_default()
    })
}

/// Raw view for tests: (content, state) of a cell.
pub fn cell(image: i32, cell: i32) -> (Vec<u8>, u8) {
    with(|v| {
        table(v, image)
            .and_then(|t| if cell < 0 { None } else { Some((t.cells.get(cell as usize)?.clone(), t.state[cell as usize])) })
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_wide_and_normalize() {
        let im = 9001;
        init(im, 8);
        store(im, 2, 0x4E00, 2);
        assert_eq!(cell(im, 2), ("一".as_bytes().to_vec(), 2));
        assert_eq!(cell(im, 3).1, 3);
        assert_eq!(span(im, 2), 2);
        assert_eq!(span(im, 3), 0);
        // overwriting the second half clears the first
        store(im, 3, b'A' as u32, 1);
        assert_eq!(cell(im, 2), (vec![], 0));
        assert_eq!(cell(im, 3), (vec![], 0));
        // cp437-representable characters leave no UTF-8
        store(im, 0, 0x2591, 1);
        assert_eq!(cell(im, 0), (vec![], 0));
        assert!(!has_content(im));
        store(im, 1, 0x0E01, 1);
        assert!(has_content(im));
        assert!(append(im, 1, 0x0E49, |_| 0));
        assert_eq!(content(im, 1), "ก้".as_bytes());
        // append to a plain cell starts from its raw byte
        assert!(append(im, 5, 0x0301, |_| b'e'));
        assert_eq!(content(im, 5), "e\u{301}".as_bytes());
        scroll_up(im, 4, 1, 2);
        assert_eq!(content(im, 1), "e\u{301}".as_bytes());
        assert_eq!(content(im, 5), b"");
        release(im);
        assert!(!append(im, 1, 0x301, |_| 0));
        assert_eq!(span(im, 1), 1);
    }

    /// Scroll and clear with rows/counts outside the table must not
    /// panic (VIEW PRINT ranges, a WIDTH change before the table is resized).
    #[test]
    fn out_of_range_scroll_and_clear_are_ignored() {
        let im = 9002;
        init(im, 8); // e.g. 4 x 2
        store(im, 6, 0x0E01, 1);
        scroll_up(im, 4, 0, 2); // top row 0: destination before the table
        scroll_up(im, 4, 1, 5); // bottom row past the table
        scroll_up(im, 4, 3, 1); // inverted range
        scroll_up(im, i32::MAX, 1, 2);
        scroll_up(im, -4, 1, 2);
        clear_range(im, 1, i32::MAX);
        clear_range(im, i32::MAX, i32::MAX);
        clear_range(im, i32::MIN, 3);
        assert_eq!(content(im, 6), b"");
        release(im);
    }
}