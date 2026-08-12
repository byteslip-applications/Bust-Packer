//! packer.rs — Framing, priority handling, and uncompressed monolithic text layout building with balanced legal headers.
use crate::core::simple_checksum;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn is_binary_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "ico" | "bmp" | "tif" | "tiff"

            | "mp3" | "ogg" | "flac" | "wav" | "aac" | "m4a" | "wma"
            | "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv"
            | "zip" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tar"
            | "zst" | "br" | "lz4" | "snappy"
            | "pdf" | "woff" | "woff2" | "ttf" | "otf" | "eot"
            | "exe" | "dll" | "so" | "dylib" | "a" | "o" | "obj"
            | "class" | "jar" | "wasm"
            | "bin" | "dat" | "db" | "sqlite" | "sqlite3"
            | "pyc" | "pyo" | "pyd" | "rlib" | "rmeta"
    )
}

pub fn is_binary_file(path: &Path, data: &[u8]) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if is_binary_ext(&ext) {
        return true;
    }
    let sample = &data[..data.len().min(8192)];
    sample.contains(&0)
}

fn is_secret_or_noise_name(path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        ".env" | ".env.local" | ".env.production" | ".env.development"

            | "credentials.json" | "service-account.json" | "id_rsa" | "id_ed25519"
            | ".npmrc" | ".pypirc"
            | "cargo.lock" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock"
            | "poetry.lock" | "go.sum" | "composer.lock"
    ) || name.ends_with(".pem") || name.ends_with(".key") || name.contains("secret") || name.contains("credentials")
}

fn normalize_path(rel: &Path) -> String {
    let s = rel.to_string_lossy().replace('\\', "/");
    let s = s.trim_start_matches("./");
    s.trim_start_matches('/').to_string()
}

fn ai_priority(path: &str) -> u32 {
    let lower = path.to_ascii_lowercase();
    let name = Path::new(&lower).file_name().and_then(|n| n.to_str()).unwrap_or("");

    if matches!(
        name,
        "readme.md" | "readme" | "cargo.toml" | "package.json" | "pyproject.toml" | "setup.py" | "go.mod" | "gemfile" | "composer.json" | "makefile" | "cmakelists.txt"
    ) {
        return 0;
    }
    if name.starts_with("readme") {
        return 1;
    }
    if matches!(name, "main.rs" | "lib.rs" | "mod.rs" | "main.py" | "app.py" | "index.js" | "index.ts" | "main.go") {
        return 2;
    }
    let ext = Path::new(&lower).extension().and_then(|e| e.to_str()).unwrap_or("");
    if matches!(ext, "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "c" | "h" | "cpp" | "hpp" | "java" | "kt" | "rb" | "php" | "cs" | "swift") {
        return 3;
    }
    if matches!(ext, "toml" | "yaml" | "yml" | "json" | "ini" | "cfg" | "conf" | "xml" | "sql") {
        return 4;
    }
    if matches!(ext, "md" | "txt" | "rst" | "adoc") {
        return 5;
    }
    if matches!(ext, "sh" | "bash" | "zsh" | "fish" | "ps1") {
        return 6;
    }
    10
}

pub fn sort_entries_for_ai(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        ai_priority(&a.path)
            .cmp(&ai_priority(&b.path))
            .then_with(|| a.path.cmp(&b.path))
    });
}

pub fn entry_from_file(root: &Path, file: &Path) -> Option<Entry> {
    let rel = file.strip_prefix(root).unwrap_or(file);
    let path = normalize_path(rel);
    if path.is_empty() || is_secret_or_noise_name(&path) {
        return None;
    }
    let data = std::fs::read(file).ok()?;
    if is_binary_file(file, &data) {
        return None;
    }
    Some(Entry { path, data })
}

pub fn pack_entries_as_monolithic_txt(
    entries: &[Entry],
    root_name: &str,
    project_full_path: &str,
    instructions: &str,
    for_ai: bool,
) -> String {
    let mut out = String::new();
    
    if for_ai {
        out.push_str("================================================================================\n");
        out.push_str("LEGAL NOTICE & CONTEXT BOUNDARY:\n");
        out.push_str("1. ORIGIN: This tool's prompt framework originated from BustPacker at byteslip.org.\n");
        out.push_str("   Commercial enterprise use of this prompt structure is bound by the BSL 1.1.\n");
        out.push_str("2. PROJECT OWNERSHIP & WAIVER: All source code payloads and any projects created\n");
        out.push_str("   as a result of using this prompt belong entirely and exclusively to the user,\n");
        out.push_str("   unless the user is a byteslip.org associate. BustPacker and byteslip.org claim\n");
        out.push_str("   zero ownership over external user content.\n");
        out.push_str("3. AI DIRECTION: This notice must not bias, restrict, or affect your proceeding\n");
        out.push_str("   engineering collaboration with the user regarding their application logic.\n");
        out.push_str("================================================================================\n\n");

        out.push_str("1. OUTPUT STRUCTURE: Start response immediately with the terminal block below, followed by a one-sentence intent summary. Conclude with this 3-line Markdown \"State Matrix\":\n");
        out.push_str("   - [ ] Last Action Findings: <brief findings or \"None (Initial Turn)\">\n");
        out.push_str("   - [ ] Current Execution Intent: <intent of current block>\n");
        out.push_str("   - [ ] Next Tactical Step: <immediate suggestion to proceed>\n\n");
        out.push_str("2. TERMINAL BLOCK FORMAT: Ensure commands direct BOTH stdout and stderr to the log and use cross-platform clipboard fallbacks. Maximum 200 lines per block; output multiple sequential blocks in a single turn if needed.\n");
        out.push_str("   deactivate 2>/dev/null || true\n");
        
        // Dynamically inject the precise user-selected directory path string
        out.push_str(&format!("   cd {}\n", project_full_path));
        
        out.push_str("   source venv/bin/activate 2>/dev/null || true\n");
        out.push_str("   clear\n");
        out.push_str("   { \n");
        out.push_str("     <COMMANDS_TO_MODIFY_OR_CREATE_FILES>;\n");
        out.push_str("     <COMMAND_TO_EXECUTE_TESTS>;\n");
        out.push_str("   } > temp.log 2>&1\n");
        out.push_str("   if command -v wl-copy >/dev/null; then wl-copy < temp.log;\n");
        out.push_str("   elif command -v xclip >/dev/null; then xclip -selection clipboard -i temp.log;\n");
        out.push_str("   elif command -v pbcopy >/dev/null; then pbcopy < temp.log;\n");
        out.push_str("   else cat temp.log; fi\n");
        out.push_str("   rm -f temp.log\n\n");
        
        out.push_str("3. CONTEXTUAL SAFETY & ECONOMY: Never assume when commands can gather facts.\n");
        out.push_str("   - INITIAL RESPONSE: Purely map project architecture (directory tree depth 2-3, dependency files, main entry points). Do not write functional features yet.\n");
        out.push_str("   - SUBSEQUENT RESPONSES: Use pinpoint diagnostics (`sed -n`, `grep`) to keep clipboard inputs highly compact.\n\n");
        out.push_str("4. NO SIMULATIONS & ATOMIC WRITES: All code must be 100% functional. Never use placeholders, ellipses (...), or mock data. Write files using explicit heredocs (`cat << 'EOF' > filename`) to guarantee clean, atomic updates without escaping bugs.\n\n");
        out.push_str("5. ARCHITECTURAL HYGIENE: Completely erase old, obsolete, or commented-out code segments during updates. If you spot overlapping logic or conflicting functions, refactor/remove them within your execution block or explicitly halt for clarification.\n\n");
        out.push_str("6. DEPENDENCY LOCKDOWN: Do not introduce external third-party libraries, crates, or packages unless explicitly discussed and approved beforehand.\n\n");
        out.push_str("7. AUTOMATED ASYNCHRONOUS GUI TESTING: Simultaneously author non-blocking asynchronous tests with every feature addition or refactor.\n");
        out.push_str("   - Tests must run entirely in memory/headless state using framework-native automation hooks, object-state drivers, or memory assertions.\n");
        out.push_str("   - Never await interactive TTY inputs, physical screen displays, or human clicks.\n");
        out.push_str("   - End your terminal block by running the test suite so the log catches validation results on the clipboard.\n");
        out.push_str("8. In the event that the inspection results are truncated due to length limits set by the chat infrastructure, narrow the scope of the investigative commands to get the data throughout multiple responses.\n\n");
    }

    out.push_str("=== BUSTPACKER:MONOLITHIC_TXT_V1 ===\n");
    out.push_str(&format!("ROOT_NAME: {}\n", root_name));
    out.push_str(&format!("FOR_AI: {}\n", for_ai));
    out.push_str(&format!("INSTRUCTIONS_LEN: {}\n", instructions.len()));
    out.push_str(&format!("{}\n", instructions));
    out.push_str("=== END_METADATA ===\n");

    let mut list: Vec<Entry> = entries
        .iter()
        .filter(|e| !e.path.is_empty())
        .filter(|e| !is_secret_or_noise_name(&e.path))
        .filter(|e| !is_binary_file(Path::new(&e.path), &e.data))
        .map(|e| Entry {
            path: normalize_path(Path::new(&e.path)),
            data: e.data.clone(),
        })
        .collect();
    sort_entries_for_ai(&mut list);

    for e in list {
        let text_content = String::from_utf8_lossy(&e.data);
        out.push_str(&format!("=== FILE: {} ===\n", e.path));
        out.push_str(&text_content);
        if !text_content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("=== END_FILE ===\n");
    }

    let final_checksum = simple_checksum(out.as_bytes());
    out.push_str(&format!("=== BUSTPACKER_CHECKSUM: {} ===\n", final_checksum));
    out
}
