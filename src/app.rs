use super::{
    metro::Metro,
    sampler::{Direction, Step, StepBuilder},
    widgets::{Layout, Page, SequencerWidget, StepEditorWidget},
};
use monome::{KeyDirection, Monome, MonomeDevice, MonomeDeviceType, MonomeEvent};
use std::{collections::HashSet, sync::mpsc::Sender};

use super::common::*;

struct Pages {
    sequencer: Page,
    step_edit: Page,
}

pub struct App {
    grid: Monome,
    current_page: Screen,
    pages: Pages,
    pressed: HashSet<usize>,
    step_index: usize,
    num_patterns: usize,
    sequence: Vec<Option<Step>>,
    sender: Sender<Step>,
}

impl App {
    pub fn new(grid: &MonomeDevice, sender: Sender<Step>) -> Result<Self, String> {
        assert_eq!(grid.device_type(), MonomeDeviceType::Grid);
        let mut grid = Monome::from_device(grid, "/prefix").map_err(|e| e.to_string())?;

        if grid.size() != (GRID_WIDTH as i32, GRID_HEIGHT as i32) {
            panic!("App only supports Grid 128 sowwy :3");
        }

        println!("Got grid 128 :3");

        grid.set_all(&[false; 128]);

        let sequencer = Page::new();
        let step_edit = Page::new();
        let mut this = App {
            grid,
            current_page: Screen::Sequencer(DEFAULT_PATTERN),
            pages: Pages {
                sequencer,
                step_edit,
            },
            pressed: HashSet::with_capacity(16),
            step_index: 0,
            sender,
            num_patterns: DEFAULT_NUM_PATTERNS,
            sequence: vec![None; SEQUENCE_LEN],
        };

        for x in 0..16 {
            SequencerWidget::Pattern(x).render(&mut this.pages.sequencer, false);
        }

        for x in 0..DEFAULT_NUM_PATTERNS {
            this.pages.sequencer.framebuffer[x] = OFF;
        }

        Ok(this)
    }

    fn write_pattern(&mut self, page: usize) {
        for x in 0..GRID_WIDTH {
            SequencerWidget::Pattern(x).render(
                &mut self.pages.sequencer,
                self.sequence[x + page * GRID_WIDTH].is_some(),
            )
        }
    }

    fn tick(&mut self) {
        if let Some(step) = self.sequence[self.step_index] {
            self.sender.send(step).unwrap()
        }

        match self.current_page {
            Screen::Sequencer(page) => {
                // First handle clearing the current step marker
                let current_page = self.step_index / GRID_WIDTH;

                if current_page == page {
                    let step_x = self.step_index % GRID_WIDTH;

                    // Just restore this step's state based on whether it has a note
                    SequencerWidget::Pattern(step_x).render(
                        &mut self.pages.sequencer,
                        self.sequence[self.step_index].is_some(),
                    )
                }

                // Advance to next step
                self.step_index = (self.step_index + 1) % self.sequence.len();

                // If new step is on current page, render the cursor
                let new_page = self.step_index / GRID_WIDTH;
                if new_page == page {
                    SequencerWidget::Pattern(self.step_index % GRID_WIDTH)
                        .render(&mut self.pages.sequencer, true)
                }

                self.pages.sequencer.render(&mut self.grid);
            }

            Screen::StepEdit { .. } => {
                self.step_index = (self.step_index + 1) % self.sequence.len();
                self.pages.step_edit.render(&mut self.grid)
            }
        }
    }

    fn handle_event(&mut self) -> bool {
        match self.grid.poll() {
            Some(event) => {
                match self.current_page {
                    Screen::Sequencer(page) => match event {
                        MonomeEvent::GridKey {
                            x,
                            y,
                            direction: KeyDirection::Down,
                        } => {
                            if let Some(widget) = SequencerWidget::hit(x as usize, y as usize) {
                                match widget {
                                    SequencerWidget::PatternSelect(selected_page) => {
                                        self.pressed.insert(selected_page);
                                        widget.render(&mut self.pages.sequencer, true);
                                    }
                                    SequencerWidget::Pattern(step) => {
                                        self.write_pattern(page);
                                        let step_builder = self.sequence[step + page * GRID_WIDTH]
                                            .and_then(|s| match s {
                                                Step::On(current_step) => Some(current_step),
                                                _ => None,
                                            })
                                            .unwrap_or_default();

                                        self.current_page = Screen::StepEdit {
                                            page,
                                            step,
                                            step_builder,
                                        };

                                        StepEditorWidget::SliceSelect(step_builder.slice())
                                            .render(&mut self.pages.step_edit, true);

                                        let dir =
                                            if let Direction::Forward = step_builder.direction() {
                                                StepEditorWidget::Forward
                                            } else {
                                                StepEditorWidget::Backward
                                            };
                                        dir.render(&mut self.pages.step_edit, true);

                                        StepEditorWidget::CurrentStep(x as usize)
                                            .render(&mut self.pages.step_edit, true);
                                    }
                                }
                            }
                        }

                        MonomeEvent::GridKey {
                            x,
                            y,
                            direction: KeyDirection::Up,
                        } => {
                            if let Some(SequencerWidget::PatternSelect(pattern)) =
                                SequencerWidget::hit(x as usize, y as usize)
                            {
                                self.pressed.remove(&pattern);
                                println!("Pressed: {:?}", self.pressed);
                                if self.pressed.is_empty() {
                                    if pattern < self.num_patterns {
                                        self.write_pattern(pattern);
                                        self.current_page = Screen::Sequencer(pattern)
                                    }
                                } else if self.pressed.contains(&0) {
                                    println!("Hit pattern select: {:?}", pattern);

                                    let added_len =
                                        (pattern + 1) * GRID_WIDTH - self.sequence.len();
                                    println!("Length being added: {:?}", added_len);

                                    self.num_patterns = pattern + 1;
                                    self.sequence
                                        .extend(std::iter::repeat(None).take(added_len));
                                    self.write_pattern(pattern)
                                }
                            }
                        }
                        _ => (),
                    },

                    Screen::StepEdit {
                        page,
                        step,
                        step_builder,
                    } => match event {
                        MonomeEvent::GridKey {
                            x,
                            y,
                            direction: KeyDirection::Down,
                        } => {
                            if let Some(widget) = StepEditorWidget::hit(x as usize, y as usize) {
                                match widget {
                                    StepEditorWidget::SliceSelect(_) => {
                                        self.current_page
                                            .set_step(step_builder.with_slice(x as usize));
                                        widget.render(&mut self.pages.step_edit, true);
                                    }
                                    StepEditorWidget::CurrentStep(_) => unreachable!(),
                                    StepEditorWidget::Backward => {
                                        self.current_page.set_step(
                                            step_builder.with_direction(Direction::Backward),
                                        );
                                        widget.render(&mut self.pages.step_edit, true);
                                    }
                                    StepEditorWidget::Forward => {
                                        self.current_page.set_step(
                                            step_builder.with_direction(Direction::Forward),
                                        );
                                        widget.render(&mut self.pages.step_edit, true);
                                    }
                                }
                            }
                        }

                        MonomeEvent::GridKey {
                            y,
                            direction: KeyDirection::Up,
                            ..
                        } => {
                            if let Some(SequencerWidget::Pattern(_)) =
                                SequencerWidget::hit(0, y as usize)
                            {
                                println!("Setting step {} to {:?}", step * page, step_builder);
                                self.sequence[step + page * GRID_WIDTH] =
                                    Some(Step::On(step_builder));

                                StepEditorWidget::CurrentStep(step)
                                    .render(&mut self.pages.step_edit, false);

                                self.current_page = Screen::Sequencer(page);
                                self.write_pattern(page);
                            }
                        }

                        _ => (),
                    },
                }
                true
            }
            _ => false,
        }
    }

    pub fn run(self) {
        println!("Starting metro :3");
        let metro = Metro::new(DEFAULT_BPM, self);
        metro.forever(App::tick, App::handle_event)
    }
}

#[derive(Debug, Clone, Copy)]
enum Screen {
    Sequencer(usize),
    StepEdit {
        page: usize,
        step: usize,
        step_builder: StepBuilder,
    },
}

impl Screen {
    fn set_step(&mut self, updated_step: StepBuilder) {
        match self {
            Self::Sequencer(_) => (),
            Self::StepEdit { page, step, .. } => {
                *self = Self::StepEdit {
                    page: *page,
                    step: *step,
                    step_builder: updated_step,
                }
            }
        }
    }
}
