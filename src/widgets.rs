use super::common::*;

pub trait Layout: Sized {
    fn hit(x: usize, y: usize) -> Option<Self>;
    fn render(&self, buffer: &mut Page, on: bool);
}

pub struct Page {
    pub framebuffer: [u8; GRID_SIZE],
}

impl Page {
    pub fn new() -> Self {
        Self {
            framebuffer: [0; GRID_SIZE],
        }
    }

    fn write_column(&mut self, x: usize, level: u8) {
        for y in 4..8 {
            self.framebuffer[to_1d(x, y)] = level;
        }
    }

    pub fn render(&self, grid: &mut monome::Monome) {
        grid.set_all_intensity(&self.framebuffer)
    }
}

pub enum StepEditorWidget {
    SliceSelect(usize),
    CurrentStep(usize),
    Forward,
    Backward,
}

impl Layout for StepEditorWidget {
    fn hit(x: usize, y: usize) -> Option<Self> {
        use StepEditorWidget::*;

        if y == 0 && (0..GRID_WIDTH).contains(&x) {
            Some(SliceSelect(x))
        } else if y == 1 {
            if (0..2).contains(&x) {
                Some(Backward)
            } else if (3..5).contains(&x) {
                Some(Forward)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn render(&self, page: &mut Page, on: bool) {
        use StepEditorWidget::*;
        match self {
            SliceSelect(index) => (0..GRID_WIDTH)
                .for_each(|idx| page.framebuffer[idx] = if idx == *index { ON } else { OFF }),
            CurrentStep(index) => page.write_column(*index, on as u8 * ON),
            Forward => {
                page.framebuffer[GRID_WIDTH..GRID_WIDTH + 2].fill(if on { OFF } else { ON });
                page.framebuffer[GRID_WIDTH + 3..GRID_WIDTH + 5].fill(if on { ON } else { OFF });
            }
            Backward => {
                page.framebuffer[GRID_WIDTH + 3..GRID_WIDTH + 5].fill(if on { OFF } else { ON });
                page.framebuffer[GRID_WIDTH..GRID_WIDTH + 2].fill(if on { ON } else { OFF });
            }
        }
    }
}

pub enum SequencerWidget {
    Pattern(usize),
    PatternSelect(usize),
}

impl Layout for SequencerWidget {
    fn hit(x: usize, y: usize) -> Option<Self> {
        if y == 0 {
            Some(SequencerWidget::PatternSelect(x))
        } else if (4 * GRID_WIDTH..128).contains(&to_1d(x, y)) {
            Some(SequencerWidget::Pattern(x))
        } else {
            None
        }
    }

    fn render(&self, page: &mut Page, on: bool) {
        use SequencerWidget::*;

        match self {
            Pattern(step) => {
                let level = if on {
                    ON
                } else if step % 4 == 0 {
                    ACCENT
                } else {
                    OFF
                };

                page.write_column(*step, level)
            }
            PatternSelect(pattern) => {
                for i in 0..*pattern + 1 {
                    page.framebuffer[i] = if i == *pattern { ON } else { OFF }
                }
            }
        }
    }
}
