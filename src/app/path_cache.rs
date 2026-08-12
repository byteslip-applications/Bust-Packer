//! path_cache.rs — Simple persistence for path memory and license agreements.
use std::fs;
use std::path::PathBuf;

fn get_cache_file() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".bustpacker_cache.txt")
}

pub fn load_last_path() -> String {
    let f = get_cache_file();
    if !f.exists() { return String::new(); }
    fs::read_to_string(f)
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn save_last_path(path: &str, agreed: bool) {
    let f = get_cache_file();
    let content = format!("{}\nagreed:{}", path, agreed);
    let _ = fs::write(f, content);
}

pub fn load_agreement_state() -> bool {
    let f = get_cache_file();
    if !f.exists() { return false; }
    let txt = fs::read_to_string(f).unwrap_or_default();
    txt.contains("agreed:true")
}

pub fn load_last_instructions() -> String {
    "AI Continuation Context Guardrails Active.".to_string()
}

pub fn save_last_instructions(_ins: &str) {}
