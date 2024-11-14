pub const DEFAULT_BPM: u32 = 172;

pub const GRID_WIDTH: usize = 16;
pub const GRID_HEIGHT: usize = 8;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;

pub const DEFAULT_NUM_PATTERNS: usize = 1;
pub const DEFAULT_PATTERN: usize = 0;
pub const SEQUENCE_LEN: usize = GRID_WIDTH * DEFAULT_NUM_PATTERNS;

pub const ON: u8 = 15;
pub const ACCENT: u8 = 8;
pub const OFF: u8 = 4;
pub const EMPTY: u8 = 0;

pub fn to_1d(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}
