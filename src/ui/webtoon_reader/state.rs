#[derive(Clone, Copy, PartialEq)]
pub enum PullDir { Top, Bottom }

pub struct WebtoonState {
    pub last_chapter: String,
    pub ref_width: f32,
    pub seek_target: Option<(usize, f32)>,
    pub just_seeked: u32,
    pub seek_page: Option<usize>,
    pub pending_seek: Option<usize>,
    pub wheel_sensitivity: f32,
    pub frame_offset: f32,
    pub velocity: f32,
    pub pull: Option<PullDir>,
    pub pull_progress: f32,
    pub near_end: bool,
    pub near_start: bool,
    pub prev_h: Vec<f32>,
    pub current_h: Vec<f32>,
    pub joined_h: Vec<f32>,
    pub last_total_h: f32,
    pub rotation_lock: u32,
    pub seek_initial_current_total: f32,
    pub last_saved_ap: String,
    pub last_saved_page: usize,
    pub last_saved_total: usize,
}

impl WebtoonState {
    pub fn new(ref_width: f32) -> Self {
        Self {
            last_chapter: String::new(),
            ref_width,
            seek_target: None,
            just_seeked: 0,
            seek_page: None,
            pending_seek: None,
            wheel_sensitivity: 1.0,
            frame_offset: 0.0,
            velocity: 0.0,
            pull: None,
            pull_progress: 0.0,
            near_end: false,
            near_start: false,
            prev_h: Vec::new(),
            current_h: Vec::new(),
            joined_h: Vec::new(),
            last_total_h: 0.0,
            rotation_lock: 0,
            seek_initial_current_total: 0.0,
            last_saved_ap: String::new(),
            last_saved_page: 0,
            last_saved_total: 0,
        }
    }
}

pub fn current_index(abs_offset: f32, cum_y: &[f32]) -> usize {
    let mut last = 0;
    for (i, &y) in cum_y.iter().enumerate() {
        if abs_offset < y { return if i == 0 { 0 } else { i - 1 }; }
        last = i;
    }
    last.saturating_sub(1)
}
