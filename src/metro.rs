use std::time::{Duration, Instant};

const LINES_PER_BAR: usize = 4;

pub struct Metro<State> {
    interval: Duration,
    last_execution: Instant,
    state: State,
}

impl<State> Metro<State> {
    pub fn new(bpm: u32, state: State) -> Self {
        let bpm_ms = 60_000. / bpm as f32;
        let interval_ms = bpm_ms / LINES_PER_BAR as f32;
        let interval = Duration::from_millis(interval_ms as u64);
        let last_execution = Instant::now();
        Self {
            interval,
            last_execution,
            state,
        }
    }

    fn is_ready(&self) -> bool {
        Instant::now() >= self.last_execution + self.interval
    }

    pub fn forever<Tick, HandleEvent>(mut self, mut tick: Tick, mut handle_event: HandleEvent)
    where
        Tick: FnMut(&mut State),
        HandleEvent: FnMut(&mut State) -> bool,
    {
        loop {
            if self.is_ready() {
                tick(&mut self.state);
                self.last_execution = Instant::now();
            } else {
                'inner: while !self.is_ready() {
                    if handle_event(&mut self.state) {
                        continue;
                    } else {
                        break 'inner;
                    }
                }
            }
        }
    }
}
