use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt; // Unix-specific permissions

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};


#[derive(Debug)]
struct Script {
    path: String,
    name: String,
    description: Option<String>,
}

// Function to extract description from a script file
// Takes a path as &str (borrowed string slice)
// returns Result<Option<String>, io::Error>
//  - Ok(Some(description)) if we found a description
//  - Ok(None) if no description
//  - Err(error) if we couldn't read the file
fn extract_description(path: &str) -> Result<Option<String>, io::Error> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        // skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // skip shebang lines (#!/bom/bash, etc.)
        if trimmed.starts_with("#!") {
            continue;
        }

        

        // Check for different comment formats
        let description = if let Some(desc) = trimmed.strip_prefix('#') {
            Some(desc)
        } else if let Some(desc) = trimmed.strip_prefix("//") {
            //found C++ style comment
            return Ok(Some(desc.trim().to_string()));
        } else if let Some(desc) = trimmed.strip_prefix("--") {
            // found lua-style comment
            return Ok(Some(desc.trim().to_string()));
        } else {
            // non comment line, stop looking
            None
        };

        if let Some(desc) = description { 
            let cleaned = desc.trim().to_string();
            if !cleaned.is_empty() {
                return Ok(Some(cleaned));
            }
            // if empty, keep looking
            continue;
        }
        
        // hit non-comment line
        break;

    }

    // no description found
    Ok(None)
}

// Scan directory for executable scripts
// takes path as &str
// returns a Rersult containing Vec<Script> or an error
fn scan_directory(directory: &str) -> Result<Vec<Script>, io::Error> {
    let entries = fs::read_dir(directory)?;
    let mut scripts: Vec<Script> = Vec::new();

    for entry_result in entries {
        let entry = entry_result?;
        let path = entry.path();

        if path.is_dir() {
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

            let script = Script {
                path: path_str,
                name,
                description,
            };

            scripts.push(script)
        }
    }
    Ok(scripts)
}


fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <directory>", args[0]);
        return Ok(());
    }

    let directory = &args[1];
    let scripts = scan_directory(directory)?;


    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    // draw UI
    terminal.draw(|f| {
        // get size of terminal
        let size = f.size();

        // create a layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(size);

        // Create title widget
        let title = Paragraph::new(format!("Script runner - {} scripts found", scripts.len()))
            .block(Block::default().borders(Borders::ALL).title("Scripts"));
        f.render_widget(title, chunks[0]);

        // Convert scripts to ListItems
        let items: Vec<ListItem> = scripts
            .iter()
            .map(|script| {
                let mut lines = vec![script.name.clone()];
                if let Some(desc) = &script.description {
                    lines.push(format!("  {}", desc));
                }
                ListItem::new(lines.join("\n"))
            })
            .collect();

        // create list widget
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Available Scripts"));
        f.render_widget(list, chunks[1]);
    })?;

    // Wait for user to press enter before exiting
    use std::io::Read;
    let mut buffer = [0; 1];
    io::stdin().read(&mut buffer)?;

    //cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
