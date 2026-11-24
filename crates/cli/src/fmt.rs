use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Canonical Syntra source code formatter
pub fn format_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent_level: usize = 0;
    let mut consecutive_blank_lines = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        // Handle blank lines: allow at most one consecutive blank line
        if trimmed.is_empty() {
            if consecutive_blank_lines == 0 && !out.is_empty() {
                out.push('\n');
                consecutive_blank_lines += 1;
            }
            continue;
        }
        consecutive_blank_lines = 0;

        // Preserve comments with proper indentation
        if trimmed.starts_with("//") {
            append_indent(&mut out, indent_level);
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // Adjust indent for closing brace before writing line
        let starts_with_close = trimmed.starts_with('}') || trimmed.starts_with(']');
        if starts_with_close && indent_level > 0 {
            indent_level -= 1;
        }

        append_indent(&mut out, indent_level);

        // Format the line tokens with proper spacing
        let formatted_line = format_line(trimmed);
        out.push_str(&formatted_line);
        out.push('\n');

        // Check if line opens a block without closing it on the same line
        let opens = count_char(trimmed, '{');
        let closes = count_char(trimmed, '}');
        if opens > closes {
            indent_level += opens - closes;
        } else if closes > opens && !starts_with_close {
            indent_level = indent_level.saturating_sub(closes - opens);
        }
    }

    out
}

fn append_indent(buf: &mut String, level: usize) {
    for _ in 0..level {
        buf.push_str("    ");
    }
}

fn count_char(s: &str, ch: char) -> usize {
    s.chars().filter(|&c| c == ch).count()
}

/// Formats a single line of Syntra code ensuring canonical spacing
fn format_line(line: &str) -> String {
    let mut res = String::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        let ch = chars[i];

        if ch == '"' && (i == 0 || chars[i - 1] != '\\') {
            in_string = !in_string;
            res.push(ch);
            i += 1;
            continue;
        }

        if in_string {
            res.push(ch);
            i += 1;
            continue;
        }

        // Commas: ensure comma is followed by space
        if ch == ',' {
            res.push(',');
            if i + 1 < len && chars[i + 1] != ' ' && chars[i + 1] != '\n' {
                res.push(' ');
            }
            i += 1;
            continue;
        }

        // Semicolons: no space before semicolon
        if ch == ';' {
            let trimmed_end = res.trim_end();
            let trailing = res.len() - trimmed_end.len();
            for _ in 0..trailing {
                res.pop();
            }
            res.push(';');
            i += 1;
            continue;
        }

        // Opening brace '{': ensure preceded by space if not at start
        if ch == '{' {
            if !res.is_empty() && !res.ends_with(' ') {
                res.push(' ');
            }
            res.push('{');
            i += 1;
            continue;
        }

        res.push(ch);
        i += 1;
    }

    res
}

/// Recursively formats a file or all .syn files in the project
pub fn format_files(target: Option<PathBuf>, check: bool) {
    let mut files = Vec::new();

    if let Some(path) = target {
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            collect_syn_files(&path, &mut files);
        }
    } else {
        collect_syn_files(Path::new("."), &mut files);
    }

    if files.is_empty() {
        println!("No Syntra (.syn) files found to format.");
        return;
    }

    let mut unformatted_count = 0;
    let mut formatted_count = 0;

    for file in &files {
        let src = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let formatted = format_source(&src);

        if src != formatted {
            unformatted_count += 1;
            if check {
                println!("⚠️  Unformatted: {}", file.display());
            } else if let Ok(()) = fs::write(file, &formatted) {
                println!("✨ Formatted: {}", file.display());
                formatted_count += 1;
            }
        }
    }

    if check {
        if unformatted_count > 0 {
            eprintln!(
                "\n❌ {unformatted_count} file(s) require formatting. Run 'syn fmt' to apply."
            );
            exit(1);
        } else {
            println!(
                "✅ All {} files conform to canonical Syntra formatting.",
                files.len()
            );
        }
    } else {
        println!(
            "🎉 Formatting complete ({} file(s) formatted out of {}).",
            formatted_count,
            files.len()
        );
    }
}

fn collect_syn_files(dir: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                collect_syn_files(&p, list);
            } else if p.extension().map(|e| e == "syn").unwrap_or(false) {
                list.push(p);
            }
        }
    }
}
