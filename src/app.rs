use super::{
    metro::Metro,
    sampler::{Direction, Step, StepBuilder},
    widgets::{Layout, Page, SequencerWidget, StepEditorWidget},
};
use monome::{KeyDirection, Monome, MonomeDevice, MonomeDeviceType, MonomeEvent};
use std::sync::mpsc::Sender;

use super::common::*;

struct Pages {
    sequencer: Page,
    step_edit: Page,
}

pub struct App {
    grid: Monome,
    screen: Screen,
    pages: Pages,
    current_index: usize,
    sequence: [Option<Step>; SEQUENCE_LEN],
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
            screen: Screen::Sequencer(DEFAULT_PAGE),
            pages: Pages {
                sequencer,
                step_edit,
            },
            current_index: 0,
            sender,
            sequence: [None; SEQUENCE_LEN],
        };

        for x in 0..16 {
            SequencerWidget::Pattern(x).render(&mut this.pages.sequencer, false);
        }

        for x in 0..PAGES {
            SequencerWidget::PageSelect(x).render(&mut this.pages.sequencer, x == DEFAULT_PAGE)
        }

        Ok(this)
    }

    fn write_page(&mut self, page: usize) {
        for x in 0..GRID_WIDTH {
            SequencerWidget::Pattern(x).render(
                &mut self.pages.sequencer,
                self.sequence[x + page * GRID_WIDTH].is_some(),
            )
        }
    }

    fn tick(&mut self) {
        if let Some(step) = self.sequence[self.current_index] {
            self.sender.send(step).unwrap()
        }

        match self.screen {
            Screen::Sequencer(page) => {
                // First handle clearing the current step marker
                let current_page = self.current_index / GRID_WIDTH;
                if current_page == page {
                    let step_x = self.current_index % GRID_WIDTH;
                    // Just restore this step's state based on whether it has a note
                    let has_note = self.sequence[self.current_index].is_some();
                    SequencerWidget::Pattern(step_x).render(&mut self.pages.sequencer, has_note)
                }

                // Advance to next step
                self.current_index = (self.current_index + 1) % SEQUENCE_LEN;

                // If new step is on current page, highlight it
                let new_page = self.current_index / GRID_WIDTH;
                if new_page == page {
                    SequencerWidget::Pattern(self.current_index % GRID_WIDTH)
                        .render(&mut self.pages.sequencer, true)
                }

                self.pages.sequencer.render(&mut self.grid);
            }

            Screen::StepEdit { .. } => {
                self.current_index = (self.current_index + 1) % SEQUENCE_LEN;
                self.pages.step_edit.render(&mut self.grid)
            }
        }
    }

    fn handle_event(&mut self) -> bool {
        match self.grid.poll() {
            Some(event) => {
                match self.screen {
                    Screen::Sequencer(page) => match event {
                        MonomeEvent::GridKey {
                            x,
                            y,
                            direction: KeyDirection::Down,
                        } => {
                            if let Some(widget) = SequencerWidget::hit(x as usize, y as usize) {
                                match widget {
                                    SequencerWidget::PageSelect(selected_page) => {
                                        self.write_page(selected_page);
                                        widget.render(&mut self.pages.sequencer, true);
                                        self.screen = Screen::Sequencer(selected_page)
                                    }
                                    SequencerWidget::Pattern(step) => {
                                        self.write_page(page);
                                        let step_builder = self.sequence[step + page * GRID_WIDTH]
                                            .and_then(|s| match s {
                                                Step::On(current_step) => Some(current_step),
                                                _ => None,
                                            })
                                            .unwrap_or_default();

                                        self.screen = Screen::StepEdit {
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
                        } if matches!(
                            SequencerWidget::hit(x as usize, y as usize),
                            Some(SequencerWidget::PageSelect(_)),
                        ) =>
                        {
                            self.write_page(page)
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
                                    StepEditorWidget::SliceSelect(slice) => {
                                        self.screen.set_step(step_builder.with_slice(x as usize));
                                        widget.render(&mut self.pages.step_edit, true);
                                    }
                                    StepEditorWidget::CurrentStep(_) => unreachable!(),
                                    StepEditorWidget::Backward => {
                                        self.screen.set_step(
                                            step_builder.with_direction(Direction::Backward),
                                        );
                                        widget.render(&mut self.pages.step_edit, true);
                                    }
                                    StepEditorWidget::Forward => {
                                        self.screen.set_step(
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

                                self.screen = Screen::Sequencer(page);
                                self.write_page(page);
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
