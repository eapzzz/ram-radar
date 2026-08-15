use std::collections::VecDeque;
use std::time::Instant;

pub struct AnimationState {
    pub history: VecDeque<(f32, Instant)>,
    pub max_history_secs: f32,
    pub animated_pss_gb: f32,
    pub animated_used_gb: f32,
    pub last_frame_time: Instant,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(120),
            max_history_secs: 60.0,
            animated_pss_gb: 0.0,
            animated_used_gb: 0.0,
            last_frame_time: Instant::now(),
        }
    }

    pub fn push_sample(&mut self, pss_gb: f32) {
        let now = Instant::now();
        self.history.push_back((pss_gb, now));
        while let Some(&(_, time)) = self.history.front() {
            if (now - time).as_secs_f32() > self.max_history_secs {
                self.history.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn update(&mut self, target_pss_gb: f32, target_used_gb: f32) {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().clamp(0.001, 0.1);
        self.last_frame_time = now;
        let alpha = (dt * 6.0).clamp(0.0, 1.0);
        self.animated_pss_gb += (target_pss_gb - self.animated_pss_gb) * alpha;
        self.animated_used_gb += (target_used_gb - self.animated_used_gb) * alpha;
    }
}
