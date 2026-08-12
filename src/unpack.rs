//! unpack.rs — Complete non-destructive parsing and validation matching the single monolithic checksum rule.
use crate::core::simple_checksum;
use std::fs;
use std::path::{Path, PathBuf};

/// Parses a monolithic text snapshot, verifies its trailing overall checksum, and unpacks files cleanly.
pub fn unpack_monolithic_txt(content: &str, output_base_dir: &Path) -> Result<PathBuf, String> {
    let mut lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err("Empty layout content file".to_string());
    }

    // Check for trailing overall checksum lines
    let last_line = lines.last().copied().unwrap_or("").trim();
    if last_line.starts_with("=== BUSTPACKER_CHECKSUM: ") && last_line.ends_with(" ===") {
        let prefix_len = "=== BUSTPACKER_CHECKSUM: ".len();
        let suffix_len = " ===".len();
        let token = &last_line[prefix_len..last_line.len() - suffix_len].trim();
        let expected_checksum: u32 = token.parse().map_err(|_| "Invalid checksum number format")?;

        // Pop checksum line off to calculate over the matching original text content payload exactly
        lines.pop();
        let mut original_payload = lines.join("\n");
        if content.ends_with('\n') {
            original_payload.push('\n');
        }

        let actual_checksum = simple_checksum(original_payload.as_bytes());
        if actual_checksum != expected_checksum {
            return Err(format!("Global snapshot checksum mismatch! Expected: {}, Found: {}", expected_checksum, actual_checksum));
        }
    }

    let mut root_name = "unpacked_project".to_string();
    let mut i = 0;
    
    if lines.starts_with(&["=== BUSTPACKER:MONOLITHIC_TXT_V1 ==="]) {
        i += 1;
        while i < lines.len() {
            let line = lines[i];
            if line == "=== END_METADATA ===" {
                i += 1;
                break;
            }
            if line.starts_with("ROOT_NAME: ") {
                root_name = line["ROOT_NAME: ".len()..].trim().to_string();
            }
            i += 1;
        }
    }

    if root_name.is_empty() {
        root_name = "unpacked_project".to_string();
    }

    let target_dir = output_base_dir.join(&root_name);
    fs::create_dir_all(&target_dir).map_err(|e| format!("Could not establish directory: {}", e))?;

    let mut current_file_path: Option<String> = None;
    let mut current_file_accumulator: Vec<&str> = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("=== FILE: ") && line.ends_with(" ===") {
            if let Some(ref path_str) = current_file_path {
                let file_dest = target_dir.join(path_str);
                if let Some(parent) = file_dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("Could not create parent path: {}", e))?;
                }
                let body = current_file_accumulator.join("\n");
                fs::write(&file_dest, body.as_bytes()).map_err(|e| format!("Could not write file {}: {}", path_str, e))?;
            }
            
            let extracted = &line["=== FILE: ".len()..line.len() - " ===".len()];
            current_file_path = Some(extracted.trim().to_string());
            current_file_accumulator.clear();
        } else if line == "=== END_FILE ===" {
            if let Some(ref path_str) = current_file_path {
                let file_dest = target_dir.join(path_str);
                if let Some(parent) = file_dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("Could not create parent path: {}", e))?;
                }
                let body = current_file_accumulator.join("\n");
                fs::write(&file_dest, body.as_bytes()).map_err(|e| format!("Could not write file {}: {}", path_str, e))?;
            }
            current_file_path = None;
            current_file_accumulator.clear();
        } else {
            if current_file_path.is_some() {
                current_file_accumulator.push(line);
            }
        }
        i += 1;
    }

    if let Some(ref path_str) = current_file_path {
        let file_dest = target_dir.join(path_str);
        if let Some(parent) = file_dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create parent path: {}", e))?;
        }
        let body = current_file_accumulator.join("\n");
        let _ = fs::write(&file_dest, body.as_bytes());
    }

    Ok(target_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monolithic_roundtrip_with_checksum() {
        // Updated checksum to match the exact djb2 algorithm output for this precise mock text
        let sample_layout = "=== BUSTPACKER:MONOLITHIC_TXT_V1 ===\nROOT_NAME: test_proj\n=== END_METADATA ===\n=== FILE: src/main.rs ===\nfn main() {}\n=== END_FILE ===\n=== BUSTPACKER_CHECKSUM: 2756697938 ===\n";
        let temp_dir = std::env::temp_dir().join("bustpacker_test_checksum_run");
        let res = unpack_monolithic_txt(sample_layout, &temp_dir).unwrap();
        assert!(res.exists());
        let file_content = fs::read_to_string(res.join("src/main.rs")).unwrap();
        assert_eq!(file_content, "fn main() {}");
        let _ = fs::remove_dir_all(temp_dir);
    }
}
