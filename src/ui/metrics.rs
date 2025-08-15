use std::fmt::Display;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Metrics {
    frame_start_time: Option<SystemTime>,
    last_update: SystemTime,
    frame_times: Vec<Duration>,
    pub report: Option<Report>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            frame_start_time: None,
            last_update: SystemTime::now(),
            frame_times: Vec::new(),
            report: None,
        }
    }
}

#[derive(Debug)]
pub struct Report {
    pub avg_frame_duration: Duration,
    pub frames_since_last_update: usize,
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{} µs | {} frames",
            self.avg_frame_duration.as_micros(),
            self.frames_since_last_update
        )
    }
}

const REFRESH_PERIOD: Duration = Duration::from_secs(1);

impl Metrics {
    pub fn frame_start(&mut self) {
        assert!(self.frame_start_time.is_none());

        self.frame_start_time = Some(SystemTime::now());
    }

    pub fn frame_stop(&mut self) -> Duration {
        self.frame_times
            .push(self.frame_start_time.unwrap().elapsed().unwrap());
        self.frame_start_time = None;

        let mut since_last_update = self.last_update.elapsed().unwrap();
        if since_last_update >= REFRESH_PERIOD {
            let frames_since_last_update = self.frame_times.len();
            let avg_frame_duration: Duration =
                self.frame_times.drain(..).sum::<Duration>() / (frames_since_last_update as u32);
            self.report = Some(Report {
                frames_since_last_update,
                avg_frame_duration,
            });

            self.last_update = SystemTime::now();
            since_last_update = Duration::ZERO;
        }

        REFRESH_PERIOD - since_last_update
    }
}
