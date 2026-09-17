use std::sync::mpsc;
use std::sync::{Condvar, Mutex, OnceLock};

use crate::archive::SharedArchive;

const MAX_TEXTURE_SIDE: u32 = 16384;

pub struct DecodeJob {
    pub ap: String,
    pub n: usize,
    pub page_name: String,
    pub max_decode_width: u32,
    pub arcs: Option<SharedArchive>,
    pub tx: mpsc::Sender<(String, usize, Vec<u8>, i32, i32, bool)>,
    pub use_max_width: bool,
}

pub fn decode_tx() -> mpsc::Sender<DecodeJob> {
    static TX: OnceLock<mpsc::Sender<DecodeJob>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<DecodeJob>();
        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let result = (|| -> Option<(Vec<u8>, i32, i32)> {
                    let (idx, zip) = job.arcs.as_ref()?;
                    let data = crate::archive::read_image_direct(idx, zip, &job.page_name, &job.ap).ok()?;
                    if job.use_max_width {
                        crate::image_processing::ImageProcessor::decode_raw_max_dimensions(&data, job.max_decode_width, MAX_TEXTURE_SIDE)
                    } else {
                        crate::image_processing::ImageProcessor::decode_raw(&data, job.max_decode_width)
                    }
                })();
                let _ = job.tx.send(match result {
                    Some((rgba, w, h)) => (job.ap, job.n, rgba, w, h, job.use_max_width),
                    _none => (job.ap, job.n, Vec::new(), 0, 0, job.use_max_width),
                });
            }
        });
        tx
    })
    .clone()
}

struct SlotCount {
    used: u32,
    max: u32,
}

impl SlotCount {
    fn acquire(&mut self) -> bool {
        if self.used < self.max {
            self.used += 1;
            true
        } else {
            false
        }
    }

    fn release(&mut self) {
        self.used = self.used.saturating_sub(1);
    }
}

fn pool() -> &'static (Mutex<SlotCount>, Condvar) {
    static POOL: OnceLock<(Mutex<SlotCount>, Condvar)> = OnceLock::new();
    POOL.get_or_init(|| {
        let max = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        (Mutex::new(SlotCount { used: 0, max: max as u32 }), Condvar::new())
    })
}

pub fn acquire_permit() {
    let (lock, cvar) = pool();
    let mut slots = lock.lock().unwrap_or_else(|e| e.into_inner());
    while !slots.acquire() {
        slots = cvar.wait(slots).unwrap_or_else(|e| e.into_inner());
    }
}

pub fn release_permit() {
    let (lock, cvar) = pool();
    let mut slots = lock.lock().unwrap_or_else(|e| e.into_inner());
    slots.release();
    cvar.notify_one();
}

pub struct PermitGuard;

impl PermitGuard {
    pub fn new() -> Self {
        Self
    }
}

impl Drop for PermitGuard {
    fn drop(&mut self) {
        release_permit();
    }
}
