use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

struct State {
    dir: Option<PathBuf>,
    index: Option<Vec<PathBuf>>,
}

fn state() -> &'static RwLock<State> {
    static S: OnceLock<RwLock<State>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(State { dir: None, index: None }))
}

fn default_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("Soundpacks")))
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("Soundpacks"))
}

fn configured_dir() -> PathBuf {
    state()
        .read()
        .unwrap()
        .dir
        .clone()
        .unwrap_or_else(default_dir)
}

pub fn set_dir(path: PathBuf) {
    let mut s = state().write().unwrap();
    s.dir = Some(path);
    s.index = None;
}

pub fn reset_dir() {
    let mut s = state().write().unwrap();
    s.dir = None;
    s.index = None;
}

pub fn dir() -> PathBuf {
    configured_dir()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("ogg") | Some("wav") | Some("mp3")
        ) {
            out.push(p);
        }
    }
}

fn ensure(s: &mut State) {
    if s.index.is_some() {
        return;
    }
    let d = s.dir.clone().unwrap_or_else(default_dir);
    let mut idx = Vec::new();
    walk(&d, &mut idx);
    idx.sort();
    s.index = Some(idx);
}

pub fn resolve(rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    if rel.is_empty() || rel.starts_with('/') || rel.split('/').any(|p| p == "..") {
        return None;
    }
    let mut s = state().write().unwrap();
    ensure(&mut s);
    let d = s.dir.clone().unwrap_or_else(default_dir);
    let exact = d.join(&rel);
    if exact.is_file() {
        return Some(exact);
    }
    let name = rel.rsplit('/').next()?.to_lowercase();
    s.index
        .as_ref()?
        .iter()
        .find(|p| {
            p.file_name()
                .map_or(false, |n| n.to_string_lossy().to_lowercase() == name)
        })
        .cloned()
}
