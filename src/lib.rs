use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt;

pub mod ui;

#[derive(Debug, PartialEq)]
pub struct Script {
    pub path: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

pub struct App {
    pub scripts: Vec<Script>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub viewing_output: bool,
    pub output_text: String,
    pub output_scroll: usize,
    pub showing_help: bool,
}

impl App {
    pub fn new(scripts: Vec<Script>) -> App {
        App {
            scripts,
            selected_index: 0,
            should_quit: false,
            viewing_output: false,
            output_text: String::new(),
            output_scroll: 0,
            showing_help: false,
        }
    }

    pub fn next(&mut self) {
        if self.selected_index < self.scripts.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn scroll_output_up(&mut self) {
        if self.output_scroll > 0 {
            self.output_scroll -= 1;
        }
    }

    pub fn scroll_output_down(&mut self, max_scroll: usize) {
        if self.output_scroll < max_scroll {
            self.output_scroll += 1;
        }
    }

    pub fn show_help(&mut self) {
        self.showing_help = true;
    }

    pub fn hide_help(&mut self) {
        self.showing_help = false;
    }

    pub fn back_to_list(&mut self) {
        self.viewing_output = false;
        self.output_text.clear();
        self.output_scroll = 0;
    }
}

pub fn extract_description(path: &str) -> Result<Option<String>, io::Error> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("#!") {
            continue;
        }

        let desc = if let Some(d) = trimmed.strip_prefix('#') {
            Some(d)
        } else if let Some(d) = trimmed.strip_prefix("//") {
            Some(d)
        } else if let Some(d) = trimmed.strip_prefix("--") {
            Some(d)
        } else {
            None
        };

        if let Some(d) = desc {
            let cleaned = d.trim().to_string();
            if !cleaned.is_empty() {
                return Ok(Some(cleaned));
            }
            continue;
        }

        break;
    }

    Ok(None)
}

pub fn scan_directory(directory: &str) -> Result<Vec<Script>, io::Error> {
    let mut scripts = Vec::new();
    scan_directory_recursive(directory, None, &mut scripts)?;
    Ok(scripts)
}

fn scan_directory_recursive(
    directory: &str,
    category: Option<String>,
    scripts: &mut Vec<Script>,
) -> Result<(), io::Error> {
    let entries = fs::read_dir(directory)?;

    for entry_result in entries {
        let entry = entry_result?;
        let path = entry.path();

        if path.is_dir() {
            let subdir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let path_str = path.to_str().unwrap_or("").to_string();

            scan_directory_recursive(&path_str, Some(subdir_name), scripts)?;
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let permissions = metadata.permissions();

        if permissions.mode() & 0o111 != 0 {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let path_str = path.to_str().unwrap_or("").to_string();

            let description = extract_description(&path_str).unwrap_or(None);

            scripts.push(Script {
                path: path_str,
                name,
                description,
                category: category.clone(),
            });
        }
    }

    Ok(())
}
