use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;
use crate::app::task::TaskMsg;
use crate::app::path_cache;
use crate::packer;

pub fn get_bpacks_dir() -> String {
    let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    p.push("bpacks");
    if !p.exists() {
        let _ = fs::create_dir_all(&p);
    }
    p.to_string_lossy().to_string()
}

pub fn get_project_internal_dirs() -> (PathBuf, PathBuf) {
    let bpacks = PathBuf::from(get_bpacks_dir());
    let bunpacks = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("bunpacks");
    if !bunpacks.exists() {
        let _ = fs::create_dir_all(&bunpacks);
    }
    (bpacks, bunpacks)
}

pub fn load_gitignore_patterns_public(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    let gi = root.join(".gitignore");
    if gi.exists() {
        if let Ok(content) = fs::read_to_string(gi) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    patterns.push(trimmed.to_string());
                }
            }
        }
    }
    patterns
}

pub fn is_ignored_path_public(rel_path: &Path, patterns: &[String]) -> bool {
    let s = rel_path.to_string_lossy().to_string();
    if s.contains("target/") || s.contains(".git/") || s.contains("venv/") || s.contains(".cargo/") {
        return true;
    }
    for p in patterns {
        if s.contains(p) {
            return true;
        }
    }
    false
}

fn collect_entries(root: &Path, progress_tx: &Sender<TaskMsg>) -> Result<(Vec<packer::Entry>, usize, String), String> {
    let mut entries = Vec::new();
    let mut bytes_total = 0;
    let mut log = format!("Scanning workspace root: {}\n", root.display());
    let gitignore_patterns = load_gitignore_patterns_public(root);

    fn walk(dir: &Path, root: &Path, patterns: &[String], entries: &mut Vec<packer::Entry>, bytes_total: &mut usize, log: &mut String, progress_tx: &Sender<TaskMsg>) -> Result<(), String> {
        if let Ok(list) = fs::read_dir(dir) {
            for entry in list.flatten() {
                let p = entry.path();
                if let Ok(rel) = p.strip_prefix(root) {
                    if is_ignored_path_public(rel, patterns) {
                        continue;
                    }
                    if p.is_dir() {
                        walk(&p, root, patterns, entries, bytes_total, log, progress_tx)?;
                    } else if p.is_file() {
                        if let Some(ent) = packer::entry_from_file(root, &p) {
                            *bytes_total += ent.data.len();
                            entries.push(ent);
                            let _ = progress_tx.send(TaskMsg::Progress {
                                files_done: entries.len(),
                                files_total: entries.len() + 1,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    walk(root, root, &gitignore_patterns, &mut entries, &mut bytes_total, &mut log, progress_tx)?;
    Ok((entries, bytes_total, log))
}

pub fn do_pack_extended(
    path: &str,
    _excludes: &[String],
    for_ai: bool,
    progress_tx: &Sender<TaskMsg>,
) -> TaskMsg {
    let start = Instant::now();
    let root = Path::new(path);
    let (entries, _, mut log) = match collect_entries(root, progress_tx) {
        Ok(v) => v,
        Err(e) => return TaskMsg::Error(e),
    };
    let files = entries.len();

    let instructions = path_cache::load_last_instructions();
    path_cache::save_last_instructions(&instructions);
    let root_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");

    let (output_dir, _) = get_project_internal_dirs();
    let bp_name = root
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("pack"))
        .to_string_lossy();

    let txt_content = packer::pack_entries_as_monolithic_txt(&entries, root_name, path, &instructions, for_ai);
    let file_suffix = if for_ai { "-Prompt.txt" } else { ".txt" };
    let final_path = output_dir.join(format!("{}{}", bp_name, file_suffix));
    let packed_data = txt_content.into_bytes();

    match fs::write(&final_path, &packed_data) {
        Ok(()) => {
            let duration_ms = start.elapsed().as_millis();
            log.push_str(&format!(
                "\n=== PACK RESULTS ===\nFiles Saved: {}\nAI Suffix: {}\nTime: {} ms\n",
                files, if for_ai { "YES (-Prompt)" } else { "NO" }, duration_ms
            ));
            TaskMsg::PackDone {
                files,
                output: final_path.display().to_string(),
                log_append: log,
            }
        }
        Err(e) => TaskMsg::Error(format!("Write failed: {}", e)),
    }
}

pub fn do_unpack(path: &str, _progress_tx: &Sender<TaskMsg>) -> TaskMsg {
    let (_, unpack_dir) = get_project_internal_dirs();
    match fs::read_to_string(path) {
        Ok(content) => match crate::unpack::unpack_monolithic_txt(&content, &unpack_dir) {
            Ok(out_p) => TaskMsg::UnpackDone {
                output: out_p.display().to_string(),
                log_append: format!("Successfully extracted snapshot package onto disk target: {}", out_p.display()),
            },
            Err(e) => TaskMsg::Error(format!("Extraction breakdown: {}", e)),
        },
        Err(e) => TaskMsg::Error(format!("Disk target read failed: {}", e)),
    }
}
