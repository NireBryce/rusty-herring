use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt; // Unix-specific permissions

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

#[derive(Debug)]
struct Script {
    path: String,
    name: String,
    description: Option<String>,
}

struct App {
    scripts: Vec<Script>,
    selected_index: usize, // which script is selected
    should_quit: bool,     // exit flag
}

impl App {
    fn new(scripts: Vec<Script>) -> App {
        App {
            scripts,
            selected_index: 0,
            should_quit: false,
        }
    }

    fn next(&mut self) {
        // move selection down
        if self.selected_index < self.scripts.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn previous(&mut self) {
        // move selection up
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }
}

// guard to ensure terminal cleanup
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn extract_description(path: &str) -> Result<Option<String>, io::Error> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("#!") {
            continue;
        }

        let description = if let Some(desc) = trimmed.strip_prefix('#') {
            Some(desc)
        } else if let Some(desc) = trimmed.strip_prefix("//") {
            Some(desc)
        } else if let Some(desc) = trimmed.strip_prefix("--") {
            Some(desc)
        } else {
            None
        };

        if let Some(desc) = description {
            let cleaned = desc.trim().to_string();
            if !cleaned.is_empty() {
                return Ok(Some(cleaned));
            }
            continue;
        }

        break;
    }

    Ok(None)
}

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

            scripts.push(script);
        }
    }
    Ok(scripts)
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<(), io::Error> {
    loop {
        // draw the UI
        terminal.draw(|f| {
            let size = f.size();
            
            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(size);
        let title_text = format!(
            "script Runner - {} scripts",
            app.scripts.len()
        );

        let title = Paragraph::new(title_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Scripts")
                    .border_style(
                        Style::default().fg(Color::Cyan)
                    )
            );
        f.render_widget(title, chunks[0]);

        // script list

        let items: Vec<ListItem> = app.scripts
            .iter()
            .enumerate()
            .map(|(i, script)|{
                let name_line = if i == app.selected_index {
                    format !("> {}", script.name)
                } else {
                    format!("  {}", script.name)
                };

                let lines = if let Some(desc) = &script.description {
                    vec![name_line, format!("   {}", desc)]
                } else {
                    vec![name_line]
                };

                let style = if i == app.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                
                ListItem::new(lines.join("\n")).style(style)
            }).collect();

        let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Available Scripts")
                        .border_style(
                            Style::default().fg(Color::Cyan)
                        )
                );
            f.render_widget(list, chunks[1]);
        
        let footer = Paragraph::new("Use arrow keys to navigate and press enter to run a script")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[2]); 
    })?;
    //handle input
    if event::poll(std::time::Duration::from_millis(100))? {}
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.quit();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.next();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.previous();
                }
                KeyCode::Enter => {
                    // for now, quit, eventually, execute
                    app.quit();
                }
                _ => {}
            }
        }

        if app.should_quit{
            break;
        }
    }
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <directory>", args[0]);
        return Ok(());
    }

    let directory = &args[1];
    let scripts = scan_directory(directory)?;

    if scripts.is_empty() {
        println!("No executable scripts found in {}", directory);
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Create guard for cleanup
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let app = App::new(scripts);
    run_app(&mut terminal, app)?;

    Ok(())
}
