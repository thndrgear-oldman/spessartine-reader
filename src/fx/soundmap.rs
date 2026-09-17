use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const TITLE_SCHEMA: u32 = 4;

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TitleMode {
    #[default]
    Manga,
    Webtoon,
}

const MAX_ACTIONS: usize = 24;
const MAX_FX: usize = 16;
const MAX_FRAMES: usize = 64;
const MAX_TRACKS: usize = 8;
const MAX_CUTS: usize = 16;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SoundMap {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub chapter: Option<u32>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub pages: BTreeMap<u32, PageMap>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TitleMap {
    pub schema: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub mode: TitleMode,
    #[serde(default)]
    pub chapters: BTreeMap<u32, SoundMap>,
}

impl TitleMap {
    pub fn load(path: &Path) -> Result<TitleMap, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut tm: TitleMap = toml::from_str(&text).map_err(|e| e.to_string())?;
        if tm.schema != TITLE_SCHEMA {
            return Err(format!("schema {}: ожидается {}", tm.schema, TITLE_SCHEMA));
        }
        for sm in tm.chapters.values_mut() {
            sanitize(sm);
        }
        Ok(tm)
    }
}

pub fn title_spfx_path_for(chapter_path: &str) -> PathBuf {
    let p = Path::new(chapter_path);
    let dir = p.parent().unwrap_or_else(|| Path::new("."));
    let stem = dir
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    dir.join(format!("{}.spfx", stem))
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PageMap {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgm: Option<SoundTrack>,
    #[serde(default, rename = "background", skip_serializing_if = "Option::is_none")]
    pub ambient: Option<SoundTrack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgm_y_start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgm_y_end: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_y_start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_y_end: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<SoundAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fx: Vec<FxCue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<Frame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<Cut>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SoundAction {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_end: Option<f32>,
}

#[derive(Serialize, Clone)]
pub struct SoundTrack {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<String>,
    #[serde(default, skip_serializing_if = "is_crossfade_default")]
    pub crossfade: f32,
    #[serde(default, skip_serializing_if = "is_volume_default")]
    pub volume: f32,
}

impl Default for SoundTrack {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            crossfade: 1.0,
            volume: 0.7,
        }
    }
}

#[derive(Deserialize)]
struct SoundTrackFull {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tracks: Vec<String>,
    #[serde(default = "default_track_crossfade")]
    crossfade: f32,
    #[serde(default = "default_track_volume")]
    volume: f32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SoundTrackDe {
    Legacy(String),
    Full(SoundTrackFull),
}

impl<'de> Deserialize<'de> for SoundTrack {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match SoundTrackDe::deserialize(d)? {
            SoundTrackDe::Legacy(s) => SoundTrack {
                tracks: if s.is_empty() { Vec::new() } else { vec![s] },
                ..Default::default()
            },
            SoundTrackDe::Full(f) => SoundTrack {
                tracks: f.tracks,
                crossfade: f.crossfade,
                volume: f.volume,
            },
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FramePlaylist {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub crossfade: f32,
}

impl Default for FramePlaylist {
    fn default() -> Self {
        Self { tracks: Vec::new(), crossfade: 0.0 }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poly: Option<Vec<[f32; 2]>>,
    #[serde(default)]
    pub order: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist: Option<FramePlaylist>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            poly: None,
            order: 0,
            playlist: None,
            enabled: true,
        }
    }
}

fn default_grid_cols() -> usize {
    3
}
fn default_grid_rows() -> usize {
    3
}

fn default_grid_points() -> Vec<[f32; 2]> {
    default_grid_points_for(3, 3)
}

fn default_grid_points_for(cols: usize, rows: usize) -> Vec<[f32; 2]> {
    let mut pts = Vec::with_capacity(cols * rows);
    for j in 0..rows {
        for i in 0..cols {
            pts.push([0.25 + i as f32 * 0.25, 0.25 + j as f32 * 0.25]);
        }
    }
    pts
}

fn default_grid_anchored() -> Vec<bool> {
    vec![true, true, true, false, false, false, false, false, false]
}

fn default_bbox() -> [f32; 4] {
    [0.25, 0.25, 0.5, 0.5]
}

fn default_mesh_detail() -> u8 {
    64
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Cut {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub poly: Vec<[f32; 2]>,
    #[serde(default = "default_grid_cols")]
    pub grid_cols: usize,
    #[serde(default = "default_grid_rows")]
    pub grid_rows: usize,
    #[serde(default = "default_grid_points")]
    pub grid_points: Vec<[f32; 2]>,
    #[serde(default = "default_grid_anchored")]
    pub grid_anchored: Vec<bool>,
    #[serde(default, rename = "grid_splits3", skip_serializing_if = "Vec::is_empty")]
    pub grid_splits: Vec<[usize; 3]>,
    #[serde(default = "default_bbox")]
    pub bbox: [f32; 4],
    #[serde(default)]
    pub fill_mode: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donor_rect: Option<[f32; 4]>,
    #[serde(default)]
    pub z_order: u8,
    #[serde(default = "default_mesh_detail")]
    pub mesh_detail: u8,
}

impl Default for Cut {
    fn default() -> Self {
        Self {
            poly: Vec::new(),
            grid_cols: default_grid_cols(),
            grid_rows: default_grid_rows(),
            grid_points: default_grid_points(),
            grid_anchored: default_grid_anchored(),
            grid_splits: Vec::new(),
            bbox: default_bbox(),
            fill_mode: 0,
            fill_color: None,
            donor_rect: None,
            z_order: 0,
            mesh_detail: default_mesh_detail(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct FxCue {
    pub kind: String,
    #[serde(default)]
    pub at: Option<f32>,
    #[serde(default)]
    pub dur: Option<f32>,
    #[serde(default)]
    pub seed: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bands: Option<u8>,
    #[serde(default)]
    pub intensity: Option<f32>,
    #[serde(default)]
    pub hz: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wavelen: Option<f32>,
    #[serde(default)]
    pub anchor: Option<u8>,
    #[serde(default)]
    pub zone: Option<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Vec<[f32; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_idx: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oneshot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Zone {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_one")]
    pub w: f32,
    #[serde(default = "default_one")]
    pub h: f32,
}

fn default_one() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

fn is_crossfade_default(v: &f32) -> bool {
    *v == default_track_crossfade()
}

fn is_volume_default(v: &f32) -> bool {
    *v == default_track_volume()
}

fn default_track_crossfade() -> f32 {
    1.0
}

fn default_track_volume() -> f32 {
    0.7
}

impl Default for Zone {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }
    }
}

const KNOWN_FX: [&str; 9] = ["rain", "flash", "flicker", "shake", "wave", "wind", "shine", "sway", "stars"];

fn sanitize_track(st: &mut Option<SoundTrack>) {
    if let Some(t) = st {
        for tr in t.tracks.iter_mut() {
            *tr = sanitize_rel(tr);
        }
        t.tracks.retain(|s| !s.is_empty());
        t.tracks.truncate(MAX_TRACKS);
        if t.volume < 0.0 || t.volume > 1.0 {
            t.volume = t.volume.clamp(0.0, 1.0);
        }
        if t.crossfade < 0.0 {
            t.crossfade = 0.0;
        }
    }
}

fn sanitize(map: &mut SoundMap) {
    for pm in map.pages.values_mut() {
        sanitize_track(&mut pm.bgm);
        sanitize_track(&mut pm.ambient);
        for y in [
            &mut pm.bgm_y_start,
            &mut pm.bgm_y_end,
            &mut pm.background_y_start,
            &mut pm.background_y_end,
        ] {
            if let Some(v) = y {
                *y = Some(v.clamp(0.0, 1.0));
            }
        }
        pm.actions.retain_mut(|a| {
            a.file = sanitize_rel(&a.file);
            !a.file.is_empty()
        });
        for a in pm.actions.iter_mut() {
            if let Some(y) = a.y_start {
                a.y_start = Some(y.clamp(0.0, 1.0));
            }
            if let Some(y) = a.y_end {
                a.y_end = Some(y.clamp(0.0, 1.0));
            }
        }
        if pm.actions.len() > MAX_ACTIONS {
            pm.actions.truncate(MAX_ACTIONS);
        }
        for c in pm.fx.iter_mut() {
            if c.kind == "wave2" {
                c.kind = "wave".to_string();
                c.axis = Some(1);
            }
        }
        pm.fx.retain(|c| KNOWN_FX.contains(&c.kind.as_str()));
        if pm.fx.len() > MAX_FX {
            pm.fx.truncate(MAX_FX);
        }
        for c in pm.fx.iter_mut() {
            if c.kind == "wind" && c.bands.is_none() {
                if let Some(h) = c.hz {
                    let b = h.round() as i64;
                    if (1..=8).contains(&b) {
                        c.bands = Some(b as u8);
                        c.hz = None;
                    }
                }
            }
            if let Some(fi) = c.frame {
                let valid = pm.frames.get(fi).map_or(false, |f| f.enabled);
                if !valid {
                    c.frame = None;
                }
            }
        }
        if pm.frames.len() > MAX_FRAMES {
            pm.frames.truncate(MAX_FRAMES);
        }
        for f in pm.frames.iter_mut() {
            if let Some(pl) = f.playlist.as_mut() {
                pl.tracks.retain(|t| !sanitize_rel(t).is_empty());
                if pl.tracks.is_empty() {
                    f.playlist = None;
                }
            }
        }
        pm.cuts.retain(|c| c.poly.len() >= 3);
        if pm.cuts.len() > MAX_CUTS {
            pm.cuts.truncate(MAX_CUTS);
        }
        for cut in pm.cuts.iter_mut() {
            cut.grid_cols = cut.grid_cols.clamp(1, 16);
            cut.grid_rows = cut.grid_rows.clamp(1, 16);
            if cut.grid_anchored.len() != cut.grid_points.len() {
                let mut a = vec![false; cut.grid_points.len()];
                use std::cmp::min;
                for i in 0..min(cut.grid_anchored.len(), cut.grid_points.len()) {
                    a[i] = cut.grid_anchored[i];
                }
                cut.grid_anchored = a;
            }
            let n_pts = cut.grid_points.len();
            cut.grid_splits.retain(|&[e, a, b]| {
                e < n_pts && a < n_pts && b < n_pts && e != a && e != b
            });
            for pt in cut.grid_points.iter_mut() {
                pt[0] = pt[0].clamp(0.0, 1.0);
                pt[1] = pt[1].clamp(0.0, 1.0);
            }
            cut.bbox[0] = cut.bbox[0].clamp(0.0, 1.0);
            cut.bbox[1] = cut.bbox[1].clamp(0.0, 1.0);
            cut.bbox[2] = cut.bbox[2].clamp(0.001, 1.0);
            cut.bbox[3] = cut.bbox[3].clamp(0.001, 1.0);
            if cut.bbox[0] + cut.bbox[2] > 1.0 { cut.bbox[2] = (1.0 - cut.bbox[0]).max(0.001); }
            if cut.bbox[1] + cut.bbox[3] > 1.0 { cut.bbox[3] = (1.0 - cut.bbox[1]).max(0.001); }
        }
        for c in pm.fx.iter_mut() {
            if let Some(ci) = c.cut_idx {
                if ci >= pm.cuts.len() {
                    c.cut_idx = None;
                }
            }
        }
    }
}

fn sanitize_rel(s: &str) -> String {
    let s = s.replace('\\', "/");
    let bad = s.is_empty()
        || s.starts_with('/')
        || s.split('/').any(|part| part == "..")
        || (s.as_bytes().first().is_some_and(|c| c.is_ascii_alphabetic())
            && s.as_bytes().get(1) == Some(&b':'));
    if bad { String::new() } else { s }
}
