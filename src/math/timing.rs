use std::{
    thread,
    time::{Duration, Instant},
};

// Based on: https://stackoverflow.com/a/33554241
// Totally obscure and not very well explained, but it works.
// I'm not sure if it's the best way to do it, but it's the only way I found.
#[derive(Debug, Clone)]
pub struct Timing {
    fps: u32,
    fps_start_time: Instant,
    fps_frame_count: u32,

    last_time: Instant,
    frame_time: f64,
}

impl Timing {
    pub fn new(tick_rate: u32) -> Timing {
        Timing {
            fps: tick_rate as u32,
            fps_start_time: Instant::now(),
            fps_frame_count: 0,
            frame_time: 0.0,
            last_time: Instant::now(),
        }
    }

    pub fn sleep(&mut self) {
        if self.fps > 60 {
            let freq = 1_000_000_000; // Nanoseconds per second
            let mut frame = Instant::now();

            while (frame.duration_since(self.fps_start_time).as_nanos() as u64 * self.fps as u64)
                < (freq * self.fps_frame_count as u64)
            {
                let sleep_time = ((self.fps_start_time.elapsed().as_nanos() as u64
                    * self.fps as u64
                    + freq * self.fps_frame_count as u64
                    - frame.elapsed().as_nanos() as u64 * self.fps as u64)
                    * 1_000
                    / (freq * self.fps as u64)) as u64;

                if sleep_time > 0 {
                    thread::sleep(Duration::from_millis(sleep_time / 1_000_000));
                }

                frame = Instant::now();
            }

            self.fps_frame_count += 1;
            if self.fps_frame_count > self.fps || self.fps_start_time.elapsed().as_secs() >= 1 {
                self.fps_frame_count = 1;
                self.fps_start_time = Instant::now();
            }

            let delta_in_seconds = self.last_time.elapsed().as_secs_f64();
            self.last_time = Instant::now();
            self.frame_time = delta_in_seconds;
        } else {
            let delta_in_seconds = self.last_time.elapsed().as_secs_f64();
            self.last_time = Instant::now();
            self.frame_time = delta_in_seconds;

            if self.fps > 0 {
                let sleep_time = (1.0 / self.fps as f64 - delta_in_seconds) * 1_000_000_000.0;
                if sleep_time > 0.0 {
                    thread::sleep(Duration::from_nanos(sleep_time as u64));
                }
            }
        }
    }

    pub fn get_fps(&self) -> u32 {
        (1.0 / self.frame_time) as u32
    }

    pub fn set_fps(&mut self, fps: u32) {
        self.fps = fps;
    }

    pub fn get_frame_time(&self) -> f64 {
        self.frame_time
    }
}
