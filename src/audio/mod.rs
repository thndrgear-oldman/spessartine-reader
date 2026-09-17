#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicU32, AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

// ── звук перелистывания (не трогаем) ──────────────────────────────────────

const SOUNDS: &[&[u8]] = &[
    include_bytes!("../../.spessartine/assets/sounds/flip1.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip2.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip3.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip4.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip5.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip6.wav"),
    include_bytes!("../../.spessartine/assets/sounds/flip7.wav"),
];

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn sink() -> Option<&'static Mutex<Sink>> {
    static SINK: OnceLock<Option<Mutex<Sink>>> = OnceLock::new();
    SINK.get_or_init(|| {
        let (stream, handle) = OutputStream::try_default().ok()?;
        let sink = Sink::try_new(&handle).ok()?;
        std::mem::forget(stream);
        Some(Mutex::new(sink))
    }).as_ref()
}

pub fn page_turn(muted: bool, volume: f32) {
    if muted { return; }
    let Some(sink) = sink() else { return };
    let idx = COUNTER.fetch_add(1, Ordering::Relaxed);
    let idx = (idx % 7).wrapping_mul(3) % 7;
    if let Ok(s) = sink.lock() {
        s.stop();
        s.set_volume(volume);
        if let Ok(src) = Decoder::new(Cursor::new(SOUNDS[idx])) {
            s.append(src);
        }
    }
}

// ── каналы fx (порт студии src/audio/mod.rs) ─────────────────────────────

// каналы: BGM (плейлист), Фон/Ambient (плейлист), 8 слотов звуковых действий (ван-шот)
// + 1 слот (8) под предпрослушивание в пикере звуков
const SFX_SLOTS: usize = 9;
pub const PREVIEW_SLOT: usize = 8;

// ── мастер-громкость (R1: SoundTrack.volume × settings.audio.volume) ────────
// Effective громкость = p.volume × master(); muted → гейт на старт слота и 0
// живьём в воркерах (при размуте игра продолжится с места).
static MASTER_VOLUME: AtomicU32 = AtomicU32::new(f32::to_bits(1.0));
static MASTER_MUTED: AtomicBool = AtomicBool::new(false);

pub fn set_master_volume(v: f32) {
    MASTER_VOLUME.store(f32::to_bits(v.clamp(0.0, 1.0)), Ordering::Relaxed);
}

pub fn set_master_muted(m: bool) {
    MASTER_MUTED.store(m, Ordering::Relaxed);
}

fn master() -> f32 {
    if MASTER_MUTED.load(Ordering::Relaxed) {
        0.0
    } else {
        f32::from_bits(MASTER_VOLUME.load(Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Bgm,
    Ambient,
}

// Эффективная громкость канала применяется вызывающей стороной
// (SoundTrack.volume × settings.audio.volume, см. fx.todo.md §15.10 R1), так что
// модуль аудио остаётся чистым воспроизводителем, без знания настроек ридера.
struct PlayerInner {
    tracks: Vec<PathBuf>,
    volume: f32,
    crossfade: f32,
    stop: bool,
    stop_fade: f32,
    // увеличивается при смене плейлиста -> воркер делает кроссфейд на новый
    revision: u64,
}

type PlayerSlot = (Arc<Mutex<PlayerInner>>, Option<JoinHandle<()>>);

struct Channels {
    handle: Option<OutputStreamHandle>,
    bgm: Option<PlayerSlot>,
    ambient: Option<PlayerSlot>,
    frame: Option<PlayerSlot>,
    sfx: Vec<Option<Sink>>,
}

fn ch() -> &'static Mutex<Channels> {
    static C: OnceLock<Mutex<Channels>> = OnceLock::new();
    C.get_or_init(|| match OutputStream::try_default() {
        Ok((stream, handle)) => {
            std::mem::forget(stream);
            Mutex::new(Channels {
                handle: Some(handle),
                bgm: None,
                ambient: None,
                frame: None,
                sfx: (0..SFX_SLOTS).map(|_| None).collect(),
            })
        }
        Err(_) => Mutex::new(Channels {
            handle: None,
            bgm: None,
            ambient: None,
            frame: None,
            sfx: (0..SFX_SLOTS).map(|_| None).collect(),
        }),
    })
}

static GENERATION: AtomicU64 = AtomicU64::new(0);

fn bump() -> u64 {
    GENERATION.fetch_add(1, Ordering::Relaxed) + 1
}

fn slot_mut(c: &mut Channels, kind: Channel) -> &mut Option<PlayerSlot> {
    match kind {
        Channel::Bgm => &mut c.bgm,
        Channel::Ambient => &mut c.ambient,
    }
}

fn ensure<'a>(
    slot: &'a mut Option<Sink>,
    handle: &Option<OutputStreamHandle>,
) -> Option<&'a mut Sink> {
    if slot.is_none() {
        *slot = handle.as_ref().and_then(|h| Sink::try_new(h).ok());
    }
    slot.as_mut()
}

fn append(sink: &mut Sink, path: &PathBuf, looped: bool) -> bool {
    let Ok(file) = File::open(path) else { return false };
    let Ok(src) = Decoder::new(BufReader::new(file)) else { return false };
    sink.stop();
    if looped {
        sink.append(src.repeat_infinite());
    } else {
        sink.append(src);
    }
    true
}

// запустить плейлист (до 8 треков).
// - при первом запуске трек начинается сразу (без фейда от тишины);
// - при смене плейлиста (другой набор треков) происходит кроссфейд;
// - при повторном вызове с тем же набором — только живое обновление громкости/кроссфейда.
pub fn play_playlist(kind: Channel, tracks: &[PathBuf], volume: f32, crossfade: f32) {
    let _ = bump();
    let mut c = ch().lock().unwrap();
    let out = c.handle.clone();
    let slot = slot_mut(&mut c, kind);
    match slot {
        None => {
            let inner = std::sync::Arc::new(Mutex::new(PlayerInner {
                tracks: tracks.to_vec(),
                volume,
                crossfade,
                stop: false,
                stop_fade: 0.0,
                revision: 1,
            }));
            let inner2 = inner.clone();
            let handle = thread::spawn(move || worker(inner2, out));
            *slot = Some((inner, Some(handle)));
        }
        Some((inner, _handle)) => {
            let mut p = inner.lock().unwrap();
            let same = p.tracks == tracks;
            p.volume = volume;
            p.crossfade = crossfade;
            if !same {
                p.tracks = tracks.to_vec();
                p.revision += 1;
            }
        }
    }
}

pub fn stop_playlist(kind: Channel, fade: f32) {
    let _ = bump();
    let mut c = ch().lock().unwrap();
    // очищаем слот: воркер (держит свой Arc) сам сделает fade-out и завершится,
    // а следующий play_playlist увидит пустой слот и поднимет новый воркер.
    if let Some((inner, _handle)) = slot_mut(&mut c, kind).take() {
        let mut p = inner.lock().unwrap();
        p.stop = true;
        p.stop_fade = fade;
    }
}

// поочерёдный однократный проход плейлиста кадра с кроссфейдом между треками.
// В отличие от BGM/Ambient не зацикливается: доиграв последний трек — завершается.
// Повторный вызов (например, открыт другой кадр) останавливает текущий и стартует новый.
pub fn play_frame(tracks: &[PathBuf], crossfade: f32) {
    let _ = bump();
    let mut c = ch().lock().unwrap();
    let out = c.handle.clone();
    // останавливаем предыдущий воркер (если он ещё жив): он сам дозатухнет и выйдет,
    // а свежий воркер ниже начнёт новый проход с чистых sink'ов.
    if let Some((inner, _)) = &c.frame {
        if let Ok(mut p) = inner.lock() {
            p.stop = true;
            p.stop_fade = 0.0;
        }
    }
    let inner = std::sync::Arc::new(Mutex::new(PlayerInner {
        tracks: tracks.to_vec(),
        volume: 1.0,
        crossfade,
        stop: false,
        stop_fade: 0.0,
        revision: 1,
    }));
    let inner2 = inner.clone();
    let handle = thread::spawn(move || frame_worker(inner2, out));
    c.frame = Some((inner, Some(handle)));
}

pub fn stop_frame(fade: f32) {
    let _ = bump();
    let mut c = ch().lock().unwrap();
    if let Some((inner, _handle)) = c.frame.take() {
        let mut p = inner.lock().unwrap();
        p.stop = true;
        p.stop_fade = fade;
    }
}

fn start_sfx(slot: usize, path: &PathBuf) -> bool {
    if slot >= SFX_SLOTS { return false; }
    // muted — гейт на старт (R1)
    if MASTER_MUTED.load(Ordering::Relaxed) { return false; }
    let mut c = ch().lock().unwrap();
    let handle = c.handle.clone();
    let Some(sink) = ensure(&mut c.sfx[slot], &handle) else { return false };
    if !append(sink, path, false) {
        return false;
    }
    sink.set_volume(master());
    true
}

pub fn play_sfx(slot: usize, path: &PathBuf) -> bool {
    let _ = bump();
    start_sfx(slot, path)
}

pub fn stop_sfx(slot: usize) {
    if slot >= SFX_SLOTS { return; }
    if let Ok(mut c) = ch().lock() {
        if let Some(sink) = c.sfx[slot].as_mut() {
            sink.stop();
        }
    }
}

/// Играет ли сейчас слот ван-шот SFX (true — звук ещё звучит).
pub fn sfx_playing(slot: usize) -> bool {
    if slot >= SFX_SLOTS {
        return false;
    }
    if let Ok(c) = ch().lock() {
        if let Some(sink) = c.sfx.get(slot).and_then(|s| s.as_ref()) {
            return !sink.empty();
        }
    }
    false
}

/// Остановить кадровый плейлист и все ван-шот SFX, НЕ трогая BGM/Ambient.
pub fn stop_frame_and_sfx() {
    let _ = bump();
    if let Ok(mut c) = ch().lock() {
        if let Some((inner, _)) = c.frame.take() {
            let mut p = inner.lock().unwrap();
            p.stop = true;
            p.stop_fade = 0.0;
        }
        for slot in c.sfx.iter_mut() {
            if let Some(sink) = slot { sink.stop(); }
        }
    }
}

pub fn stop_all() {
    let _ = bump();
    stop_playlist(Channel::Bgm, 0.0);
    stop_playlist(Channel::Ambient, 0.0);
    stop_frame_and_sfx();
}

#[allow(unused_assignments)]
fn worker(inner: std::sync::Arc<Mutex<PlayerInner>>, out: Option<OutputStreamHandle>) {
    let mut sink_a = out.as_ref().and_then(|h| Sink::try_new(h).ok());
    let mut sink_b = out.as_ref().and_then(|h| Sink::try_new(h).ok());
    let mut active_a = true;
    let mut idx = 0usize;
    let mut playing = false;
    let mut current_dur = Duration::from_secs(10);

    loop {
        let (tracks, volume, crossfade, stop, stop_fade, revision) = {
            let p = inner.lock().unwrap();
            (
                p.tracks.clone(),
                p.volume,
                p.crossfade,
                p.stop,
                p.stop_fade,
                p.revision,
            )
        };

        if stop {
            let active = if active_a { sink_a.as_ref() } else { sink_b.as_ref() };
            if let Some(s) = active {
                ramp(Some(s), volume * master(), 0.0, Duration::from_secs_f32(stop_fade.max(0.0)));
            }
            return;
        }

        if tracks.is_empty() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        if idx >= tracks.len() {
            idx = 0;
        }

        if !playing {
            let active = if active_a { sink_a.as_mut() } else { sink_b.as_mut() };
            let mut loaded = false;
            if let Some(s) = active {
                if let Some(path) = tracks.get(idx) {
                    if let Some(d) = load_track(s, path) {
                        current_dur = d;
                        loaded = true;
                    }
                }
            }
            if loaded {
                playing = true;
            } else {
                idx = (idx + 1) % tracks.len();
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        }

        let cf = crossfade.max(0.0);
        let wait = current_dur
            .saturating_sub(Duration::from_secs_f32(cf))
            .max(Duration::from_millis(40));
        let step = Duration::from_millis(40);
        let mut el = Duration::ZERO;
        let mut revision_changed = false;
        while el < wait {
            thread::sleep(step);
            el += step;
            let p = inner.lock().unwrap();
            if p.stop {
                return;
            }
            // живая громкость
            if let Some(s) = if active_a { sink_a.as_ref() } else { sink_b.as_ref() } {
                s.set_volume(p.volume * master());
            }
            if p.revision != revision {
                revision_changed = true;
                break;
            }
        }

        // кроссфейд на следующий трек (или на новый плейлист при смене revision)
        let (new_tracks, new_cf) = {
            let p = inner.lock().unwrap();
            (p.tracks.clone(), p.crossfade.max(0.0))
        };
        if new_tracks.is_empty() {
            playing = false;
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        let next_idx = if revision_changed { 0 } else { (idx + 1) % new_tracks.len() };
        let cf2 = new_cf.max(0.0);
        let mut next_dur = current_dur;
        {
            let inactive = if active_a { sink_b.as_mut() } else { sink_a.as_mut() };
            if let Some(s) = inactive {
                if let Some(path) = new_tracks.get(next_idx) {
                    if let Some(d) = load_track(s, path) {
                        next_dur = d;
                    }
                }
            }
        }
        // сам кроссфейд: активный затухает, неактивный нарастает
        let steps = ((cf2 / 0.04).ceil().max(1.0)) as usize;
        for i in 0..=steps {
            let p = i as f32 / steps as f32;
            let m = master();
            if let Some(s) = if active_a { sink_a.as_ref() } else { sink_b.as_ref() } {
                s.set_volume((volume * (1.0 - p) * m).max(0.0));
            }
            if let Some(s) = if active_a { sink_b.as_ref() } else { sink_a.as_ref() } {
                s.set_volume((volume * p * m).max(0.0));
            }
            thread::sleep(step);
        }
        // остановить старый активный
        if active_a {
            sink_a.as_mut().map(|s| s.stop());
        } else {
            sink_b.as_mut().map(|s| s.stop());
        }
        active_a = !active_a;
        idx = next_idx;
        current_dur = next_dur;
        playing = true;
    }
}

fn track_duration(path: &PathBuf) -> Option<Duration> {
    let file = File::open(path).ok()?;
    let mut src = Decoder::new(BufReader::new(file)).ok()?;
    if let Some(d) = src.total_duration() {
        return Some(d);
    }
    let rate = u64::from(src.sample_rate());
    let ch = u64::from(src.channels());
    if rate == 0 || ch == 0 {
        return None;
    }
    let mut samples: u64 = 0;
    while src.next().is_some() {
        samples += 1;
    }
    if samples == 0 {
        return None;
    }
    Some(Duration::from_secs_f64(samples as f64 / (rate * ch) as f64))
}

fn load_track(sink: &mut Sink, path: &PathBuf) -> Option<Duration> {
    let dur = track_duration(path);
    let file = File::open(path).ok()?;
    let src = Decoder::new(BufReader::new(file)).ok()?;
    sink.stop();
    sink.append(src);
    dur
}

// добавить трек в конец sink без остановки текущего (для последовательного прохода)
fn append_src(sink: &mut Sink, path: &PathBuf) -> bool {
    let Ok(file) = File::open(path) else { return false };
    let Ok(src) = Decoder::new(BufReader::new(file)) else { return false };
    sink.append(src);
    true
}

// линейный затух/нарастание громкости sink за dur (dur=0 — мгновенно)
fn ramp(sink: Option<&Sink>, from: f32, to: f32, dur: Duration) {
    if dur <= Duration::from_millis(0) {
        if let Some(s) = sink {
            s.set_volume(to.max(0.0).min(1.0));
        }
        return;
    }
    let step = Duration::from_millis(40);
    let steps = ((dur.as_secs_f32() / 0.04).ceil().max(1.0)) as usize;
    for i in 0..=steps {
        let p = i as f32 / steps as f32;
        if let Some(s) = sink {
            s.set_volume((from + (to - from) * p).max(0.0).min(1.0));
        }
        thread::sleep(step);
    }
}

// поочерёдный однократный проход плейлиста кадра: кроссфейд между соседними
// треками (crossfade) и остановка после последнего (без зацикливания).
#[allow(unused_assignments)]
fn frame_worker(inner: std::sync::Arc<Mutex<PlayerInner>>, out: Option<OutputStreamHandle>) {
    let Some(mut sink) = out.as_ref().and_then(|h| Sink::try_new(h).ok()) else {
        return;
    };
    let step = Duration::from_millis(40);

    let (tracks, crossfade, stop, stop_fade, revision) = {
        let p = inner.lock().unwrap();
        (
            p.tracks.clone(),
            p.crossfade,
            p.stop,
            p.stop_fade,
            p.revision,
        )
    };
    if stop {
        return;
    }

    // кроссфейд <= 0: надёжный последовательный проход — все треки аппендятся в один
    // sink, который сам играет их по очереди (первый закончился -> следующий).
    if crossfade <= 0.0 {
        let mut any = false;
        for p in &tracks {
            if append_src(&mut sink, p) {
                any = true;
            }
        }
        if !any {
            return;
        }
        sink.set_volume(master());
        while !sink.empty() {
            thread::sleep(step);
            let p = inner.lock().unwrap();
            if p.stop {
                ramp(
                    Some(&sink),
                    master(),
                    0.0,
                    Duration::from_secs_f32(stop_fade.max(0.0)),
                );
                return;
            }
            // живая мастер-громкость
            sink.set_volume(master());
            // плейлист заменён новым воркером — этот завершается
            if p.revision != revision || p.tracks != tracks {
                return;
            }
        }
        return;
    }

    // кроссфейд > 0: двух-sink плавный переход между соседними треками
    let mut sink_a = out.as_ref().and_then(|h| Sink::try_new(h).ok());
    let mut sink_b = out.as_ref().and_then(|h| Sink::try_new(h).ok());
    let mut active_a = true;
    let mut idx = 0usize;
    let mut playing = false;
    let mut current_dur = Duration::from_secs(10);

    loop {
        let (tracks, cf, stop, stop_fade, revision) = {
            let p = inner.lock().unwrap();
            (
                p.tracks.clone(),
                p.crossfade.max(0.0),
                p.stop,
                p.stop_fade,
                p.revision,
            )
        };
        if stop {
            let active = if active_a { sink_a.as_ref() } else { sink_b.as_ref() };
            if let Some(s) = active {
                ramp(Some(s), master(), 0.0, Duration::from_secs_f32(stop_fade.max(0.0)));
            }
            return;
        }
        if cf <= 0.0 {
            // кроссфейд сброшен на 0 во время проигрывания — завершаем этот воркер
            return;
        }
        if idx >= tracks.len() {
            return;
        }
        if !playing {
            let active = if active_a { sink_a.as_mut() } else { sink_b.as_mut() };
            let mut loaded = false;
            if let Some(s) = active {
                if let Some(path) = tracks.get(idx) {
                    if let Some(d) = load_track(s, path) {
                        current_dur = d;
                        loaded = true;
                    }
                }
            }
            if loaded {
                playing = true;
            } else {
                idx += 1;
                thread::sleep(step);
                continue;
            }
        }
        let has_next = idx + 1 < tracks.len();
        let wait = if has_next {
            current_dur
                .saturating_sub(Duration::from_secs_f32(cf))
                .max(Duration::from_millis(40))
        } else {
            current_dur
        };
        let mut el = Duration::ZERO;
        let mut revision_changed = false;
        while el < wait {
            thread::sleep(step);
            el += step;
            let p = inner.lock().unwrap();
            if p.stop {
                return;
            }
            if let Some(s) = if active_a { sink_a.as_ref() } else { sink_b.as_ref() } {
                s.set_volume(master());
            }
            if p.revision != revision {
                revision_changed = true;
                break;
            }
        }
        if revision_changed {
            return;
        }
        if has_next {
            let next = idx + 1;
            let next_dur = {
                let inactive = if active_a { sink_b.as_mut() } else { sink_a.as_mut() };
                if let Some(s) = inactive {
                    if let Some(path) = tracks.get(next) {
                        load_track(s, path).unwrap_or(current_dur)
                    } else {
                        current_dur
                    }
                } else {
                    current_dur
                }
            };
            let steps = ((cf / 0.04).ceil().max(1.0)) as usize;
            for i in 0..=steps {
                let p = i as f32 / steps as f32;
                let m = master();
                if let Some(s) = if active_a { sink_a.as_ref() } else { sink_b.as_ref() } {
                    s.set_volume((1.0 - p) * m);
                }
                if let Some(s) = if active_a { sink_b.as_ref() } else { sink_a.as_ref() } {
                    s.set_volume(p * m);
                }
                thread::sleep(step);
            }
            if active_a {
                sink_a.as_mut().map(|s| s.stop());
            } else {
                sink_b.as_mut().map(|s| s.stop());
            }
            active_a = !active_a;
            idx = next;
            current_dur = next_dur;
            playing = true;
        } else {
            return;
        }
    }
}
