mod decode;
mod metro;
mod sampler;
mod stream;
use cpal::traits::StreamTrait;
use metro::Metro;
use monome::{KeyDirection, Monome, MonomeDevice, MonomeDeviceType, MonomeEvent};
use sampler::{Direction, Sampler, Step, StepBuilder};
use std::{ops::RangeBounds, path::Path, sync::mpsc::Sender};

const DEFAULT_BPM: u32 = 172;
const PAGES: usize = 2;
const DEFAULT_PAGE: usize = 0;
const SEQUENCE_LEN: usize = GRID_WIDTH * PAGES;

const ON: u8 = 15;
const ACCENT: u8 = 8;
const OFF: u8 = 4;
const EMPTY: u8 = 0;

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

struct Screens {
    sequencer: [u8; 128],
    step_edit: [u8; 128],
}

impl Screens {
    fn write_step_edit_widget(&mut self, widget: StepEditorWidget, on: bool) {
        use StepEditorWidget::*;
        match widget {
            SliceSelect(index) => (0..GRID_WIDTH)
                .for_each(|idx| self.step_edit[idx] = if idx == index { ON } else { OFF }),
            CurrentStep(index) => write_column(&mut self.step_edit, index, on as u8 * ON),
            Forward => {
                self.step_edit[GRID_WIDTH..GRID_WIDTH + 2].fill(if on { OFF } else { ON });
                self.step_edit[GRID_WIDTH + 3..GRID_WIDTH + 5].fill(if on { ON } else { OFF });
            }
            Backward => {
                self.step_edit[GRID_WIDTH + 3..GRID_WIDTH + 5].fill(if on { OFF } else { ON });
                self.step_edit[GRID_WIDTH..GRID_WIDTH + 2].fill(if on { ON } else { OFF });
            }
        }
    }

    fn write_sequencer_widget(&mut self, widget: SequencerWidget, on: bool) {
        use SequencerWidget::*;

        match widget {
            Pattern(step) => {
                let level = if on {
                    ON
                } else if step % 4 == 0 {
                    ACCENT
                } else {
                    OFF
                };

                write_column(&mut self.sequencer, step, level)
            }
            PageSelect(page) => {
                for i in 0..PAGES {
                    if i == page {
                        self.sequencer[i] = ON;
                    } else {
                        self.sequencer[i] = OFF;
                    }
                }
            }
        }
    }
}

struct App {
    grid: Monome,
    screen: Screen,
    screens: Screens,
    current_index: usize,
    sequence: [Option<Step>; SEQUENCE_LEN],
    sender: Sender<Step>,
}

const GRID_WIDTH: usize = 16;

fn to_2d(idx: usize) -> (usize, usize) {
    (idx % GRID_WIDTH, idx / GRID_WIDTH)
}

fn to_1d(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}

fn in_sequencer_row(idx: i32) -> bool {
    (4..8).contains(&idx)
}

enum StepEditorWidget {
    SliceSelect(usize),
    CurrentStep(usize),
    Forward,
    Backward,
}

fn in_step_editor(x: i32, y: i32) -> Option<StepEditorWidget> {
    use StepEditorWidget::*;

    if y == 0 && (0..GRID_WIDTH as i32).contains(&x) {
        Some(SliceSelect(x as usize))
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

enum SequencerWidget {
    Pattern(usize),
    PageSelect(usize),
}

fn in_sequencer(x: usize, y: usize) -> Option<SequencerWidget> {
    if y == 0 && x < 4 {
        Some(SequencerWidget::PageSelect(x))
    } else if (4 * GRID_WIDTH..128).contains(&to_1d(x, y)) {
        Some(SequencerWidget::Pattern(x))
    } else {
        None
    }
}

fn write_column(screen: &mut [u8; 128], x: usize, level: u8) {
    for y in 4..8 {
        screen[to_1d(x, y)] = level;
    }
}

impl App {
    fn new(grid: &MonomeDevice, sender: Sender<Step>) -> Result<Self, String> {
        assert_eq!(grid.device_type(), MonomeDeviceType::Grid);
        let mut grid = Monome::from_device(grid, "/prefix").map_err(|e| e.to_string())?;
        assert_eq!(grid.size(), (16, 8));
        println!("Got grid 128 :3");

        grid.set_all(&[false; 128]);

        let sequencer = [0u8; 128];
        let step_edit = [0u8; 128];
        let mut this = App {
            grid,
            screen: Screen::Sequencer(DEFAULT_PAGE),
            screens: Screens {
                sequencer,
                step_edit,
            },
            current_index: 0,
            sender,
            sequence: [None; SEQUENCE_LEN],
        };

        for x in 0..16 {
            this.screens
                .write_sequencer_widget(SequencerWidget::Pattern(x), false);
        }

        for x in 0..PAGES {
            this.screens
                .write_sequencer_widget(SequencerWidget::PageSelect(x), x == DEFAULT_PAGE)
        }

        Ok(this)
    }

    fn write_page(&mut self, page: usize) {
        for x in 0..GRID_WIDTH {
            self.screens.write_sequencer_widget(
                SequencerWidget::Pattern(x),
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
                    self.screens
                        .write_sequencer_widget(SequencerWidget::Pattern(step_x), has_note);
                }

                // Advance to next step
                self.current_index = (self.current_index + 1) % SEQUENCE_LEN;

                // If new step is on current page, highlight it
                let new_page = self.current_index / GRID_WIDTH;
                if new_page == page {
                    self.screens.write_sequencer_widget(
                        SequencerWidget::Pattern(self.current_index % GRID_WIDTH),
                        true, // Could use a different brightness for current step
                    );
                }

                self.grid.set_all_intensity(&self.screens.sequencer);
            }

            Screen::StepEdit { .. } => {
                self.current_index = (self.current_index + 1) % SEQUENCE_LEN;
                self.grid.set_all_intensity(&self.screens.step_edit)
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
                            if let Some(widget) = in_sequencer(x as usize, y as usize) {
                                match widget {
                                    SequencerWidget::PageSelect(selected_page) => {
                                        self.write_page(selected_page);
                                        self.screens.write_sequencer_widget(
                                            SequencerWidget::PageSelect(selected_page),
                                            true,
                                        );
                                        self.screen = Screen::Sequencer(selected_page)
                                    }
                                    SequencerWidget::Pattern(step) => {
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

                                        self.screens.write_step_edit_widget(
                                            StepEditorWidget::SliceSelect(step_builder.slice()),
                                            true,
                                        );

                                        self.screens.write_step_edit_widget(
                                            if let Direction::Forward = step_builder.direction() {
                                                StepEditorWidget::Forward
                                            } else {
                                                StepEditorWidget::Backward
                                            },
                                            true,
                                        );

                                        self.screens.write_step_edit_widget(
                                            StepEditorWidget::CurrentStep(x as usize),
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        MonomeEvent::GridKey {
                            x,
                            y,
                            direction: KeyDirection::Up,
                        } => {
                            if let Some(SequencerWidget::PageSelect(_)) =
                                in_sequencer(x as usize, y as usize)
                            {
                                self.write_page(page);
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
                            if let Some(widget) = in_step_editor(x, y) {
                                match widget {
                                    StepEditorWidget::SliceSelect(slice) => {
                                        self.screen.set_step(step_builder.with_slice(x as usize));
                                        self.screens.write_step_edit_widget(
                                            StepEditorWidget::SliceSelect(slice),
                                            true,
                                        );
                                    }
                                    StepEditorWidget::CurrentStep(_) => unreachable!(),
                                    StepEditorWidget::Backward => {
                                        self.screen.set_step(
                                            step_builder.with_direction(Direction::Backward),
                                        );
                                        self.screens.write_step_edit_widget(
                                            StepEditorWidget::Backward,
                                            true,
                                        );
                                    }
                                    StepEditorWidget::Forward => {
                                        self.screen.set_step(
                                            step_builder.with_direction(Direction::Forward),
                                        );
                                        self.screens.write_step_edit_widget(
                                            StepEditorWidget::Forward,
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        MonomeEvent::GridKey {
                            y,
                            direction: KeyDirection::Up,
                            ..
                        } => {
                            if in_sequencer_row(y) {
                                println!("Setting step {} to {:?}", step * page, step_builder);
                                self.sequence[step + page * GRID_WIDTH] =
                                    Some(Step::On(step_builder));
                                self.screens.write_step_edit_widget(
                                    StepEditorWidget::CurrentStep(step),
                                    false,
                                );

                                self.screen = Screen::Sequencer(page);
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

    fn run(self) {
        println!("Starting metro :3");
        let metro = Metro::new(DEFAULT_BPM, self);
        metro.forever(App::tick, App::handle_event)
    }
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    let path = Path::new("amen.wav");
    let (samples, _frames_collected) = decode::decode(path);
    let total_len = samples.len();
    println!("Got {} samples", total_len);
    let (sender, receiver) = std::sync::mpsc::channel::<Step>();
    let sample_player = Sampler::new(samples, receiver);

    let stream = stream::setup(sample_player).unwrap();
    stream.play().unwrap();
    match monome::Monome::enumerate_devices().as_deref() {
        Ok([grid]) => match App::new(grid, sender) {
            Ok(state) => state.run(),
            Err(e) => {
                println!("Setup failed: {e}");
            }
        },
        Ok(_) => println!("Grid not found :3"),
        Err(e) => panic!("Monome error: {e}"),
    }

    Ok(())
}
