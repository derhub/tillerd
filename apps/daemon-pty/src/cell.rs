//! Cell encoding: language-neutral wire contract.

use serde::Serialize;

pub const ATTR_BOLD: u16 = 0x01;
pub const ATTR_DIM: u16 = 0x02;
pub const ATTR_ITALIC: u16 = 0x04;
pub const ATTR_UNDERLINE: u16 = 0x08;
pub const ATTR_BLINK: u16 = 0x10;
pub const ATTR_INVERSE: u16 = 0x20;
pub const ATTR_INVISIBLE: u16 = 0x40;

/// Closed integer color scheme:
///   0          = default
///   1–8        = ANSI standard (30–37 -> 1–8)
///   9–16       = ANSI bright (90–97 -> 9–16)
///   17–272     = 256-color (index + 17)
///   0x1000000+ = 24-bit RGB (0x1000000 | r<<16 | g<<8 | b)
pub const COLOR_DEFAULT: u32 = 0;

pub fn ansi_to_color(n: i64, bright: bool) -> u32 {
    if bright {
        (n - 90 + 9) as u32
    } else {
        (n - 30 + 1) as u32
    }
}

pub fn color_256(idx: i64) -> u32 {
    (idx + 17) as u32
}

pub fn color_rgb(r: i64, g: i64, b: i64) -> u32 {
    0x1000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SnapshotCell {
    pub char: String,
    pub fg: u32,
    pub bg: u32,
    pub attrs: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SnapshotPayload {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<SnapshotCell>>,
    pub cursor: Cursor,
}

pub fn char_display_width(cp: u32) -> u8 {
    if (0x1100..=0x115f).contains(&cp)
        || (0x2e80..=0x303f).contains(&cp)
        || (0x3040..=0x33ff).contains(&cp)
        || (0x3400..=0x4dbf).contains(&cp)
        || (0x4e00..=0x9fff).contains(&cp)
        || (0xa960..=0xa97f).contains(&cp)
        || (0xac00..=0xd7ff).contains(&cp)
        || (0xf900..=0xfaff).contains(&cp)
        || (0xfe10..=0xfe1f).contains(&cp)
        || (0xfe30..=0xfe4f).contains(&cp)
        || (0xfe50..=0xfe6f).contains(&cp)
        || (0xff01..=0xff60).contains(&cp)
        || (0xffe0..=0xffe6).contains(&cp)
        || (0x1b000..=0x1bfff).contains(&cp)
        || (0x1c000..=0x1cfff).contains(&cp)
        || (0x20000..=0x2fffd).contains(&cp)
        || (0x30000..=0x3fffd).contains(&cp)
    {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_helpers_match_contract() {
        assert_eq!(ansi_to_color(31, false), 2); // red
        assert_eq!(ansi_to_color(91, true), 10); // bright red
        assert_eq!(color_256(0), 17);
        assert_eq!(color_256(255), 272);
        assert_eq!(color_rgb(0xff, 0x80, 0x00), 0x1000000 | 0xff8000);
    }

    #[test]
    fn wide_chars() {
        assert_eq!(char_display_width('A' as u32), 1);
        assert_eq!(char_display_width('世' as u32), 2);
        assert_eq!(char_display_width('가' as u32), 2);
    }
}
