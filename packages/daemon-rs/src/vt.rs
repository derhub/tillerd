//! Virtual-terminal grid model. Direct 1:1 port of the reference `vt-state.ts`
//! hand-rolled parser, chosen over an off-the-shelf VT crate so snapshot output
//! matches the reference daemon cell-for-cell (the conformance-critical seam).

use crate::cell::{
    ansi_to_color, char_display_width, color_256, color_rgb, Cursor, SnapshotCell, SnapshotPayload,
    ATTR_BLINK, ATTR_BOLD, ATTR_DIM, ATTR_INVERSE, ATTR_INVISIBLE, ATTR_ITALIC, ATTR_UNDERLINE,
    COLOR_DEFAULT,
};

// `Copy` so make_grid / erase / scroll never touch the heap — the hot path that
// dominated snapshot-build time when each cell held a String. `ch` is the single
// codepoint in the cell; `wide_cont` marks the trailing half of a wide char,
// which serializes to "" per the wire contract.
#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    fg: u32,
    bg: u32,
    attrs: u16,
    wide_cont: bool,
}

fn empty_cell() -> Cell {
    Cell {
        ch: ' ',
        fg: COLOR_DEFAULT,
        bg: COLOR_DEFAULT,
        attrs: 0,
        wide_cont: false,
    }
}

fn make_grid(rows: usize, cols: usize) -> Vec<Vec<Cell>> {
    vec![vec![empty_cell(); cols]; rows]
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ParserState {
    Normal,
    Esc,
    Csi,
    Osc,
    Dcs,
}

pub struct VtState {
    rows: usize,
    cols: usize,
    grid: Vec<Vec<Cell>>,
    alt_grid: Option<Vec<Vec<Cell>>>,
    cursor: Cursor,
    saved_cursor: Cursor,
    alt_cursor: Cursor,
    in_alt_screen: bool,
    fg: u32,
    bg: u32,
    attrs: u16,

    parser_state: ParserState,
    param_buf: String,
    is_private: bool,
    inter_buf: String,
    utf8_buf: [u8; 4],
    utf8_len: usize,
    utf8_needed: usize,
}

impl VtState {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: make_grid(rows, cols),
            alt_grid: None,
            cursor: Cursor { x: 0, y: 0 },
            saved_cursor: Cursor { x: 0, y: 0 },
            alt_cursor: Cursor { x: 0, y: 0 },
            in_alt_screen: false,
            fg: COLOR_DEFAULT,
            bg: COLOR_DEFAULT,
            attrs: 0,
            parser_state: ParserState::Normal,
            param_buf: String::new(),
            is_private: false,
            inter_buf: String::new(),
            utf8_buf: [0; 4],
            utf8_len: 0,
            utf8_needed: 0,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            i += 1;

            // UTF-8 continuation
            if self.utf8_needed > 0 {
                if (b & 0xc0) == 0x80 {
                    self.utf8_buf[self.utf8_len] = b;
                    self.utf8_len += 1;
                    if self.utf8_len == self.utf8_needed {
                        let c = std::str::from_utf8(&self.utf8_buf[..self.utf8_len])
                            .ok()
                            .and_then(|s| s.chars().next())
                            .unwrap_or('\u{20}');
                        self.utf8_len = 0;
                        self.utf8_needed = 0;
                        self.process_char(c);
                    }
                    continue;
                }
                // Invalid continuation — reset and reprocess this byte.
                self.utf8_len = 0;
                self.utf8_needed = 0;
            }

            // UTF-8 start byte (only in NORMAL state)
            if self.parser_state == ParserState::Normal && (b & 0x80) != 0 {
                if (b & 0xe0) == 0xc0 {
                    self.utf8_buf[0] = b;
                    self.utf8_len = 1;
                    self.utf8_needed = 2;
                    continue;
                }
                if (b & 0xf0) == 0xe0 {
                    self.utf8_buf[0] = b;
                    self.utf8_len = 1;
                    self.utf8_needed = 3;
                    continue;
                }
                if (b & 0xf8) == 0xf0 {
                    self.utf8_buf[0] = b;
                    self.utf8_len = 1;
                    self.utf8_needed = 4;
                    continue;
                }
                continue; // invalid
            }

            self.process_byte(b);
        }
    }

    fn process_byte(&mut self, b: u8) {
        match self.parser_state {
            ParserState::Normal => self.process_normal(b),
            ParserState::Esc => self.process_esc(b),
            ParserState::Csi => self.process_csi(b),
            ParserState::Osc | ParserState::Dcs => {
                if b == 0x07 || b == 0x9c {
                    self.parser_state = ParserState::Normal;
                } else if b == 0x1b {
                    self.parser_state = ParserState::Esc;
                }
            }
        }
    }

    fn process_normal(&mut self, b: u8) {
        match b {
            0x1b => self.parser_state = ParserState::Esc,
            0x0d => self.cursor.x = 0,
            0x0a..=0x0c => {
                // LF / VT / FF
                self.cursor.y += 1;
                if self.cursor.y >= self.rows {
                    self.scroll_up();
                    self.cursor.y = self.rows - 1;
                }
            }
            0x08 => {
                if self.cursor.x > 0 {
                    self.cursor.x -= 1;
                }
            }
            0x09 => {
                let next = ((self.cursor.x / 8) + 1) * 8;
                self.cursor.x = next.min(self.cols - 1);
            }
            0x07 | 0x00 => {}
            _ => {
                if b >= 0x20 {
                    self.process_char(b as char);
                }
            }
        }
    }

    fn process_char(&mut self, c: char) {
        let width = char_display_width(c as u32) as usize;

        if self.cursor.x >= self.cols {
            self.cursor.x = 0;
            self.cursor.y += 1;
            if self.cursor.y >= self.rows {
                self.scroll_up();
                self.cursor.y = self.rows - 1;
            }
        }

        let (fg, bg, attrs) = (self.fg, self.bg, self.attrs);
        let (y, x) = (self.cursor.y, self.cursor.x);
        let g = self.active_grid_mut();
        if let Some(cell) = g.get_mut(y).and_then(|r| r.get_mut(x)) {
            cell.ch = c;
            cell.fg = fg;
            cell.bg = bg;
            cell.attrs = attrs;
            cell.wide_cont = false;
        }

        if width == 2 && x + 1 < self.cols {
            let g = self.active_grid_mut();
            if let Some(cont) = g.get_mut(y).and_then(|r| r.get_mut(x + 1)) {
                cont.ch = ' ';
                cont.fg = fg;
                cont.bg = bg;
                cont.attrs = attrs;
                cont.wide_cont = true; // serializes to "" (wide-char continuation)
            }
        }

        self.cursor.x += width;
    }

    fn process_esc(&mut self, b: u8) {
        self.parser_state = ParserState::Normal;
        match b {
            0x5b => {
                self.parser_state = ParserState::Csi;
                self.param_buf.clear();
                self.is_private = false;
                self.inter_buf.clear();
            }
            0x5d => self.parser_state = ParserState::Osc,
            0x50 => self.parser_state = ParserState::Dcs,
            0x37 => self.save_cursor(),
            0x38 => self.restore_cursor(),
            0x4d => {
                if self.cursor.y == 0 {
                    self.scroll_down();
                } else {
                    self.cursor.y -= 1;
                }
            }
            0x63 => self.full_reset(),
            0x5c => {}
            _ => {}
        }
    }

    fn process_csi(&mut self, b: u8) {
        if (0x30..=0x3f).contains(&b) {
            if b == 0x3f {
                self.is_private = true;
            } else {
                self.param_buf.push(b as char);
            }
        } else if (0x20..=0x2f).contains(&b) {
            self.inter_buf.push(b as char);
        } else if (0x40..=0x7e).contains(&b) {
            self.dispatch_csi(b);
            self.parser_state = ParserState::Normal;
        } else {
            self.parser_state = ParserState::Normal;
        }
    }

    fn dispatch_csi(&mut self, final_byte: u8) {
        let params = self.parse_params();

        if self.is_private {
            self.dispatch_private(final_byte, &params);
            return;
        }

        let p0 = params.first().copied().unwrap_or(0);
        let p1 = params.get(1).copied().unwrap_or(0);
        let max1 = |p: i64| p.max(1);

        match final_byte {
            0x41 => self.cursor.y = self.cursor.y.saturating_sub(max1(p0) as usize),
            0x42 => self.cursor.y = (self.cursor.y + max1(p0) as usize).min(self.rows - 1),
            0x43 => self.cursor.x = (self.cursor.x + max1(p0) as usize).min(self.cols - 1),
            0x44 => self.cursor.x = self.cursor.x.saturating_sub(max1(p0) as usize),
            0x45 => {
                self.cursor.y = (self.cursor.y + max1(p0) as usize).min(self.rows - 1);
                self.cursor.x = 0;
            }
            0x46 => {
                self.cursor.y = self.cursor.y.saturating_sub(max1(p0) as usize);
                self.cursor.x = 0;
            }
            0x47 => self.cursor.x = ((max1(p0) - 1).max(0) as usize).min(self.cols - 1),
            0x48 | 0x66 => {
                self.cursor.y = ((max1(p0) - 1).max(0) as usize).min(self.rows - 1);
                self.cursor.x = ((max1(p1) - 1).max(0) as usize).min(self.cols - 1);
            }
            0x4a => self.erase_display(p0),
            0x4b => self.erase_line(p0),
            0x4c => self.insert_lines(max1(p0) as usize),
            0x4d => self.delete_lines(max1(p0) as usize),
            0x50 => self.delete_chars(max1(p0) as usize),
            0x53 => {
                for _ in 0..max1(p0) {
                    self.scroll_up();
                }
            }
            0x54 => {
                for _ in 0..max1(p0) {
                    self.scroll_down();
                }
            }
            0x58 => self.erase_chars(max1(p0) as usize),
            0x6d => self.process_sgr(&params),
            0x72 => {}
            0x73 => self.save_cursor(),
            0x75 => self.restore_cursor(),
            0x64 => self.cursor.y = ((max1(p0) - 1).max(0) as usize).min(self.rows - 1),
            _ => {}
        }
    }

    fn dispatch_private(&mut self, final_byte: u8, params: &[i64]) {
        let p0 = params.first().copied().unwrap_or(0);
        // 1049 = alternate screen buffer (set via `h`, reset via `l`).
        match (final_byte, p0) {
            (0x68, 1049) => self.enter_alt_screen(),
            (0x6c, 1049) => self.exit_alt_screen(),
            _ => {}
        }
    }

    fn parse_params(&self) -> Vec<i64> {
        if self.param_buf.is_empty() {
            return Vec::new();
        }
        self.param_buf
            .split(';')
            .map(|s| {
                if s.is_empty() {
                    0
                } else {
                    s.parse::<i64>().unwrap_or(0)
                }
            })
            .collect()
    }

    fn process_sgr(&mut self, params: &[i64]) {
        let owned;
        let params: &[i64] = if params.is_empty() {
            owned = vec![0i64];
            &owned
        } else {
            params
        };
        let mut i = 0;
        while i < params.len() {
            let p = params[i];
            match p {
                0 => {
                    self.fg = COLOR_DEFAULT;
                    self.bg = COLOR_DEFAULT;
                    self.attrs = 0;
                }
                1 => self.attrs |= ATTR_BOLD,
                2 => self.attrs |= ATTR_DIM,
                3 => self.attrs |= ATTR_ITALIC,
                4 => self.attrs |= ATTR_UNDERLINE,
                5 | 6 => self.attrs |= ATTR_BLINK,
                7 => self.attrs |= ATTR_INVERSE,
                8 => self.attrs |= ATTR_INVISIBLE,
                22 => self.attrs &= !(ATTR_BOLD | ATTR_DIM),
                23 => self.attrs &= !ATTR_ITALIC,
                24 => self.attrs &= !ATTR_UNDERLINE,
                25 => self.attrs &= !ATTR_BLINK,
                27 => self.attrs &= !ATTR_INVERSE,
                28 => self.attrs &= !ATTR_INVISIBLE,
                30..=37 => self.fg = ansi_to_color(p, false),
                38 => {
                    let (color, consumed) = self.parse_sgr_color(params, i + 1);
                    self.fg = color;
                    i += consumed + 1;
                    continue;
                }
                39 => self.fg = COLOR_DEFAULT,
                40..=47 => self.bg = ansi_to_color(p - 10, false),
                48 => {
                    let (color, consumed) = self.parse_sgr_color(params, i + 1);
                    self.bg = color;
                    i += consumed + 1;
                    continue;
                }
                49 => self.bg = COLOR_DEFAULT,
                90..=97 => self.fg = ansi_to_color(p, true),
                100..=107 => self.bg = ansi_to_color(p - 10, true),
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_sgr_color(&self, params: &[i64], start: usize) -> (u32, usize) {
        let mode = params.get(start).copied();
        if mode == Some(5) {
            let idx = params.get(start + 1).copied().unwrap_or(0);
            return (color_256(idx), 2);
        }
        if mode == Some(2) {
            let r = params.get(start + 1).copied().unwrap_or(0);
            let g = params.get(start + 2).copied().unwrap_or(0);
            let b = params.get(start + 3).copied().unwrap_or(0);
            return (color_rgb(r, g, b), 4);
        }
        (COLOR_DEFAULT, 1)
    }

    fn erase_display(&mut self, mode: i64) {
        let blank = empty_cell();
        let (rows, cols, cx, cy) = (self.rows, self.cols, self.cursor.x, self.cursor.y);
        match mode {
            0 => {
                let g = self.active_grid_mut();
                g[cy][cx..cols].fill(blank);
                for row in &mut g[(cy + 1)..rows] {
                    row.fill(blank);
                }
            }
            1 => {
                let g = self.active_grid_mut();
                for row in &mut g[0..cy] {
                    row.fill(blank);
                }
                g[cy][0..=cx].fill(blank);
            }
            2 => {
                if self.in_alt_screen && self.alt_grid.is_some() {
                    self.alt_grid = Some(make_grid(rows, cols));
                } else {
                    self.grid = make_grid(rows, cols);
                }
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: i64) {
        let blank = empty_cell();
        let (cols, cx, cy) = (self.cols, self.cursor.x, self.cursor.y);
        let row = &mut self.active_grid_mut()[cy];
        match mode {
            0 => row[cx..cols].fill(blank),
            1 => row[0..=cx].fill(blank),
            2 => row[0..cols].fill(blank),
            _ => {}
        }
    }

    fn erase_chars(&mut self, n: usize) {
        let blank = empty_cell();
        let (cols, cx, cy) = (self.cols, self.cursor.x, self.cursor.y);
        let row = &mut self.active_grid_mut()[cy];
        row[cx..(cx + n).min(cols)].fill(blank);
    }

    fn insert_lines(&mut self, n: usize) {
        let (rows, cols, cy) = (self.rows, self.cols, self.cursor.y);
        let g = self.active_grid_mut();
        for _ in 0..n {
            g.remove(rows - 1);
            g.insert(cy, vec![empty_cell(); cols]);
        }
    }

    fn delete_lines(&mut self, n: usize) {
        let (cols, cy) = (self.cols, self.cursor.y);
        let g = self.active_grid_mut();
        for _ in 0..n {
            g.remove(cy);
            g.push(vec![empty_cell(); cols]);
        }
    }

    fn delete_chars(&mut self, n: usize) {
        let (cols, cx, cy) = (self.cols, self.cursor.x, self.cursor.y);
        let g = self.active_grid_mut();
        let row = &mut g[cy];
        for _ in 0..n {
            if cx < row.len() {
                row.remove(cx);
            }
        }
        while row.len() < cols {
            row.push(empty_cell());
        }
    }

    fn scroll_up(&mut self) {
        let cols = self.cols;
        let g = self.active_grid_mut();
        g.remove(0);
        g.push(vec![empty_cell(); cols]);
    }

    fn scroll_down(&mut self) {
        let cols = self.cols;
        let g = self.active_grid_mut();
        g.pop();
        g.insert(0, vec![empty_cell(); cols]);
    }

    fn enter_alt_screen(&mut self) {
        if self.in_alt_screen {
            return;
        }
        self.alt_grid = Some(make_grid(self.rows, self.cols));
        self.alt_cursor = self.cursor.clone();
        self.in_alt_screen = true;
        self.cursor = Cursor { x: 0, y: 0 };
    }

    fn exit_alt_screen(&mut self) {
        if !self.in_alt_screen {
            return;
        }
        self.alt_grid = None;
        self.in_alt_screen = false;
        self.cursor = self.alt_cursor.clone();
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = self.cursor.clone();
    }

    fn restore_cursor(&mut self) {
        self.cursor = self.saved_cursor.clone();
    }

    fn full_reset(&mut self) {
        self.grid = make_grid(self.rows, self.cols);
        self.alt_grid = None;
        self.in_alt_screen = false;
        self.cursor = Cursor { x: 0, y: 0 };
        self.saved_cursor = Cursor { x: 0, y: 0 };
        self.alt_cursor = Cursor { x: 0, y: 0 };
        self.fg = COLOR_DEFAULT;
        self.bg = COLOR_DEFAULT;
        self.attrs = 0;
    }

    fn active_grid_mut(&mut self) -> &mut Vec<Vec<Cell>> {
        if self.in_alt_screen {
            if let Some(g) = self.alt_grid.as_mut() {
                return g;
            }
        }
        &mut self.grid
    }

    fn active_grid(&self) -> &Vec<Vec<Cell>> {
        if self.in_alt_screen {
            if let Some(g) = self.alt_grid.as_ref() {
                return g;
            }
        }
        &self.grid
    }

    pub fn get_snapshot(&self) -> SnapshotPayload {
        let g = self.active_grid();
        let cells: Vec<Vec<SnapshotCell>> = g
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| SnapshotCell {
                        char: if c.wide_cont {
                            String::new()
                        } else {
                            c.ch.to_string()
                        },
                        fg: c.fg,
                        bg: c.bg,
                        attrs: c.attrs,
                    })
                    .collect()
            })
            .collect();
        SnapshotPayload {
            rows: self.rows,
            cols: self.cols,
            cells,
            cursor: self.cursor.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_at(snap: &SnapshotPayload, row: usize) -> String {
        snap.cells[row]
            .iter()
            .map(|c| c.char.as_str())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn plain_text_lands_on_grid() {
        let mut vt = VtState::new(5, 20);
        vt.feed(b"hello");
        let snap = vt.get_snapshot();
        assert_eq!(text_at(&snap, 0), "hello");
        assert_eq!(snap.cursor, Cursor { x: 5, y: 0 });
    }

    #[test]
    fn crlf_moves_cursor() {
        let mut vt = VtState::new(5, 20);
        vt.feed(b"ab\r\ncd");
        let snap = vt.get_snapshot();
        assert_eq!(text_at(&snap, 0), "ab");
        assert_eq!(text_at(&snap, 1), "cd");
        assert_eq!(snap.cursor, Cursor { x: 2, y: 1 });
    }

    #[test]
    fn sgr_red_foreground() {
        let mut vt = VtState::new(2, 10);
        vt.feed(b"\x1b[31mX");
        let snap = vt.get_snapshot();
        assert_eq!(snap.cells[0][0].char, "X");
        assert_eq!(snap.cells[0][0].fg, 2); // ANSI red -> 2
    }

    #[test]
    fn erase_display_clears() {
        let mut vt = VtState::new(3, 5);
        vt.feed(b"aaaaa\r\nbbbbb");
        vt.feed(b"\x1b[2J");
        let snap = vt.get_snapshot();
        assert_eq!(text_at(&snap, 0), "");
        assert_eq!(text_at(&snap, 1), "");
    }

    #[test]
    fn wide_char_continuation_is_empty_string() {
        let mut vt = VtState::new(2, 10);
        vt.feed("世".as_bytes());
        let snap = vt.get_snapshot();
        assert_eq!(snap.cells[0][0].char, "世");
        assert_eq!(snap.cells[0][1].char, ""); // continuation
        assert_eq!(snap.cursor.x, 2);
    }

    #[test]
    fn cursor_position_csi() {
        let mut vt = VtState::new(10, 10);
        vt.feed(b"\x1b[3;5H");
        let snap = vt.get_snapshot();
        assert_eq!(snap.cursor, Cursor { x: 4, y: 2 });
    }

    /// Golden parity: the same input fed to the reference `vt-state.ts` produces
    /// the fixture in tests/fixtures/vt-golden.json. The Rust port must reproduce
    /// its cells and cursor exactly. The input literal here is byte-identical to
    /// the one used to generate the fixture.
    #[test]
    fn snapshot_parity_with_reference() {
        let input: &str = "Hello\r\n\
            \x1b[1;31mBOLD-RED\x1b[0m\r\n\
            \x1b[38;5;200m256\x1b[0m \x1b[48;2;10;20;30mRGB\x1b[0m\r\n\
            \x1b[4munder\x1b[24m wide:世界\r\n\
            \x1b[2;3Hmoved";
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/vt-golden.json")).unwrap();

        let mut vt = VtState::new(
            golden["rows"].as_u64().unwrap() as usize,
            golden["cols"].as_u64().unwrap() as usize,
        );
        vt.feed(input.as_bytes());
        let snap = vt.get_snapshot();
        let mine = serde_json::to_value(&snap).unwrap();

        assert_eq!(mine["rows"], golden["rows"]);
        assert_eq!(mine["cols"], golden["cols"]);
        assert_eq!(mine["cursor"], golden["cursor"], "cursor diverged");
        assert_eq!(
            mine["cells"], golden["cells"],
            "cell grid diverged from reference"
        );
    }
}
