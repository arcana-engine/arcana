use {crate::clocks::TimeSpan, std::collections::VecDeque};

pub struct FpsMeter {
    frames: VecDeque<TimeSpan>,
    total: TimeSpan,
}

impl FpsMeter {
    pub fn new() -> Self {
        FpsMeter {
            frames: VecDeque::new(),
            total: TimeSpan::ZERO,
        }
    }

    pub fn add_frame_time(&mut self, span: TimeSpan) {
        self.frames.push_back(span);
        self.total += span;

        while self.total > TimeSpan::SECOND * 5 {
            let span = self.frames.pop_front().unwrap();
            self.total -= span;
        }
    }

    pub fn fps(&self) -> f32 {
        if self.frames.is_empty() {
            0.0
        } else {
            self.frames.len() as f32 / self.total.as_secs_f32()
        }
    }
}
