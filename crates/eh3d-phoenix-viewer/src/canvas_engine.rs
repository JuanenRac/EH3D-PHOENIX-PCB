// SPDX-License-Identifier: GPL-3.0-or-later
//! The engine that draws the primitives of the board in the canvas, and the measurement that decides whether
//! it is good enough, without a screen.
//!
//! The decision record of the interface stack chooses egui with the wgpu backend, with a canvas drawn by our
//! own code. Until that engine is written the window draws with the painter of egui, behind the one
//! interface that both share (the trait of the window, `CanvasEngine`): changing the engine is another
//! implementation of that trait and the setting `canvas_engine`, not a change of the viewer. This module
//! holds the parts that need no window: the names of the engines, which of them are built, what is done
//! with a request for one that is not, and the frame times with the verdict of the decision record
//! (a 95th percentile under 17 ms on the medium board and under 33 ms on the large board, on integrated
//! graphics).

use std::collections::VecDeque;

/// The engines the canvas may be drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Engine {
    /// The painter of egui: it tessellates on the processor at every frame.
    #[default]
    Painter,
    /// A callback of `egui_wgpu` fed by the primitives of the scene: not written yet.
    Wgpu,
}

impl Engine {
    /// Every engine, in the order the settings file lists them.
    pub const ALL: [Engine; 2] = [Engine::Painter, Engine::Wgpu];

    /// The word of the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Engine::Painter => "painter",
            Engine::Wgpu => "wgpu",
        }
    }

    /// The engine a word of the settings file names.
    pub fn from_key(text: &str) -> Option<Engine> {
        Engine::ALL.into_iter().find(|e| e.key() == text.trim())
    }

    /// Whether the engine is part of this build.
    pub fn is_built(self) -> bool {
        matches!(self, Engine::Painter)
    }
}

/// The engine that will be used and whether it is not the one that was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The engine that draws.
    pub engine: Engine,
    /// True when the engine asked for is not built and the painter was taken instead.
    pub fell_back: bool,
}

/// The engine to use for a request: the one asked for when it is built, otherwise the painter.
pub fn choose(asked: Engine) -> Choice {
    if asked.is_built() {
        Choice {
            engine: asked,
            fell_back: false,
        }
    } else {
        Choice {
            engine: Engine::Painter,
            fell_back: true,
        }
    }
}

/// The largest 95th percentile of the frame time of the medium board that the decision record accepts, in milliseconds.
pub const P95_MEDIUM_MS: f64 = 17.0;
/// The largest 95th percentile of the frame time of the large board that the decision record accepts, in milliseconds.
pub const P95_LARGE_MS: f64 = 33.0;
/// The longest answer to a click that the decision record accepts, in milliseconds.
pub const CLICK_MS: f64 = 100.0;

/// The value below which `percent` per cent of the times are (the nearest rank method), or `None` when there
/// is none. A time that is not a number is not counted.
pub fn percentile(times_ms: &[f64], percent: f64) -> Option<f64> {
    let mut v: Vec<f64> = times_ms.iter().copied().filter(|t| t.is_finite()).collect();
    if v.is_empty() || !percent.is_finite() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let p = percent.clamp(0.0, 100.0);
    let rank = ((p / 100.0) * v.len() as f64).ceil() as usize;
    Some(v[rank.clamp(1, v.len()) - 1])
}

/// The last frame times, as many as the setting `frame_log_frames` allows.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameLog {
    times_ms: VecDeque<f64>,
    max: usize,
}

/// The numbers that the decision record asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    /// The median frame time, in milliseconds.
    pub median_ms: f64,
    /// The 95th percentile.
    pub p95_ms: f64,
    /// The 99th percentile.
    pub p99_ms: f64,
    /// How many frames the numbers are of.
    pub frames: usize,
}

impl FrameStats {
    /// Whether the 95th percentile is within a limit (one of [`P95_MEDIUM_MS`] and [`P95_LARGE_MS`]).
    pub fn within(&self, limit_ms: f64) -> bool {
        self.p95_ms <= limit_ms
    }
}

impl FrameLog {
    /// A log that keeps at most `max_frames` times (at least one).
    pub fn new(max_frames: usize) -> FrameLog {
        FrameLog {
            times_ms: VecDeque::new(),
            max: max_frames.max(1),
        }
    }

    /// Writes the time of a frame, in milliseconds; a time that is not a number or is below zero is not written.
    pub fn push(&mut self, ms: f64) {
        if !ms.is_finite() || ms < 0.0 {
            return;
        }
        while self.times_ms.len() >= self.max {
            self.times_ms.pop_front();
        }
        self.times_ms.push_back(ms);
    }

    /// How many times are kept.
    pub fn len(&self) -> usize {
        self.times_ms.len()
    }

    /// Whether no time is kept.
    pub fn is_empty(&self) -> bool {
        self.times_ms.is_empty()
    }

    /// The median, the 95th and the 99th percentile of the times kept, or `None` when there are none.
    pub fn stats(&self) -> Option<FrameStats> {
        let times: Vec<f64> = self.times_ms.iter().copied().collect();
        Some(FrameStats {
            median_ms: percentile(&times, 50.0)?,
            p95_ms: percentile(&times, 95.0)?,
            p99_ms: percentile(&times, 99.0)?,
            frames: times.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generator of numbers with a fixed seed, so the test tries the same cases every time.
    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed >> 33
    }

    #[test]
    fn the_engines_have_words_that_read_back() {
        for e in Engine::ALL {
            assert_eq!(Engine::from_key(e.key()), Some(e));
            assert_eq!(Engine::from_key(&format!("  {}\t", e.key())), Some(e));
        }
        assert_eq!(Engine::from_key("glow"), None);
        assert_eq!(Engine::from_key(""), None);
        assert_eq!(Engine::default(), Engine::Painter);
    }

    #[test]
    fn only_the_painter_is_built_and_a_request_for_another_falls_back_to_it() {
        assert!(Engine::Painter.is_built());
        assert!(!Engine::Wgpu.is_built());
        assert_eq!(
            choose(Engine::Painter),
            Choice {
                engine: Engine::Painter,
                fell_back: false
            }
        );
        assert_eq!(
            choose(Engine::Wgpu),
            Choice {
                engine: Engine::Painter,
                fell_back: true
            }
        );
        // whatever is asked for, the engine that is chosen is built
        for e in Engine::ALL {
            assert!(choose(e).engine.is_built());
        }
    }

    #[test]
    fn the_percentile_is_the_nearest_rank() {
        let t: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&t, 50.0), Some(50.0));
        assert_eq!(percentile(&t, 95.0), Some(95.0));
        assert_eq!(percentile(&t, 99.0), Some(99.0));
        assert_eq!(percentile(&t, 100.0), Some(100.0));
        assert_eq!(percentile(&t, 0.0), Some(1.0));
        assert_eq!(percentile(&[7.0], 95.0), Some(7.0));
        assert_eq!(percentile(&[], 50.0), None);
        assert_eq!(percentile(&[f64::NAN, f64::INFINITY], 50.0), None);
        assert_eq!(percentile(&t, f64::NAN), None);
        // the order of the times does not matter
        let mut shuffled = t.clone();
        shuffled.reverse();
        assert_eq!(percentile(&shuffled, 95.0), Some(95.0));
    }

    #[test]
    fn the_log_keeps_the_last_frames_and_ignores_what_is_not_a_time() {
        let mut log = FrameLog::new(3);
        assert!(log.is_empty());
        assert!(log.stats().is_none());
        for t in [1.0, 2.0, 3.0, 4.0, f64::NAN, -1.0, f64::INFINITY] {
            log.push(t);
        }
        assert_eq!(log.len(), 3);
        let s = log.stats().unwrap();
        assert_eq!(s.frames, 3);
        assert_eq!(s.median_ms, 3.0);
        assert_eq!(s.p99_ms, 4.0);
        assert_eq!(FrameLog::new(0).max, 1);
    }

    #[test]
    fn the_verdict_follows_the_95th_percentile() {
        let mut fast = FrameLog::new(1000);
        let mut slow = FrameLog::new(1000);
        for i in 0..100 {
            fast.push(8.0 + f64::from(i % 3));
            // one frame in ten takes 40 ms
            slow.push(if i % 10 == 0 { 40.0 } else { 10.0 });
        }
        let fast = fast.stats().unwrap();
        let slow = slow.stats().unwrap();
        assert!(fast.within(P95_MEDIUM_MS) && fast.within(P95_LARGE_MS));
        // the slow log has a median of 10 ms but a 95th percentile of 40 ms
        assert_eq!(slow.median_ms, 10.0);
        assert!(!slow.within(P95_MEDIUM_MS) && !slow.within(P95_LARGE_MS));
        const { assert!(P95_MEDIUM_MS < P95_LARGE_MS) };
    }

    #[test]
    fn what_is_drawn_from_a_seed_keeps_the_order_of_the_percentiles() {
        let mut seed = 0xf8a3e_u64;
        for max in [1_usize, 10, 600] {
            let mut log = FrameLog::new(max);
            for i in 0..2000 {
                log.push((lcg(&mut seed) % 5000) as f64 / 100.0);
                assert!(log.len() <= max);
                if i % 97 == 0 {
                    let s = log.stats().unwrap();
                    assert!(s.median_ms <= s.p95_ms && s.p95_ms <= s.p99_ms, "{s:?}");
                    assert_eq!(s.frames, log.len());
                }
            }
        }
    }
}
