pub const DEFAULT_BPM: u32 = 172;
pub const PAGES: usize = 2;

pub const GRID_WIDTH: usize = 16;
pub const GRID_HEIGHT: usize = 8;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;

pub const DEFAULT_PAGE: usize = 0;
pub const SEQUENCE_LEN: usize = GRID_WIDTH * PAGES;

pub const ON: u8 = 15;
pub const ACCENT: u8 = 8;
pub const OFF: u8 = 4;
pub const EMPTY: u8 = 0;

pub fn to_1d(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}
