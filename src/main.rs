use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen
    },
    execute,
};

#[derive(Debug)]
struct Script {
    path: String,
    name: String,
    description: Option<String>,
}

struct App {
    scripts: Vec<Script>,
    selected_index: usize,
    should_quit: bool,
    viewing_output: bool,
    output_text: String,
    output_scroll: usize,
    showing_help: bool,
}

impl App {
    fn new(scripts: Vec<Script>) -> App {
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

    fn show_help(&mut self) {
        self.showing_help = true;
    }

    fn hide_help(&mut self) {
        self.showing_help = false;
    }

    fn render_help_view(f: &mut ratatui::Frame) {
        let size = f.size();

        let chunks = { 
            Layout::default()
                .direction(Direction::Vertical)
                .Constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3),])
                .split(size)
        };
        let title = { 
            Paragraph::new("Keyboard Shortcuts")
             .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(
                        Style::default().fg(Color::Yellow)
                    )
            )
        };
        f.render_widget(title, chunks[0]);
    
        // Help content
        let help_text = "\
            Script List View:
            ↑/k         - Move selection up
            ↓/j         - Move selection down
            Enter       - Run selected script
            ?           - Show this help
            q/Esc       - Quit application

            Output View:
            ↑/k         - Scroll up
            ↓/j         - Scroll down
            Any other   - Return to script list

            General:
            All commands are case-sensitive
            Navigation uses vim-style keys (j/k) or arrows
        ";

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default().fg(Color::Yellow)
                    )
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(help, chunks[1]);
        
        // Footer
        let footer = Paragraph::new("Press any key to close")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default().fg(Color::Yellow)
                    )
            )
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[2]);
    }
    fn scroll_output_up(&mut self) {
        if self.output_scroll > 0 { 
            self.output_scroll -= 1;
        }
    }

    fn scroll_output_down(&mut self, max_scroll: usize) {
        if self.output_scroll < max_scroll {
            self.output_scroll += 1;
        }
    }

    fn back_to_list(&mut self) {
        self.viewing_output = false;
        self.output_text.clear();
        self.output_scroll = 0;
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
fn render_list_view(
    f: &mut ratatui::Frame,
    app: &App,
) {
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
        "Script Runner - {} scripts",
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
    
    let items: Vec<ListItem> = app.scripts
        .iter()
        .enumerate()
        .map(|(i, script)| {
            let prefix = if i == app.selected_index {
                "▶"
            } else {
                " "
            };
            
            let name_line = format!(
                "{} {}",
                prefix,
                script.name
            );
            
            let lines = if let Some(d) = &script.description {
                vec![name_line, format!("    {}", d)]
            } else {
                vec![name_line]
            };
            
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(lines.join("\n")).style(style)
        })
        .collect();
    
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
    
    let footer = Paragraph::new(
        "↑/↓: Navigate | Enter: Run | q: Quit"
    )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(Color::Cyan)
                )
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
}

fn render_output_view(
    f: &mut ratatui::Frame,
    app: &App,
) {
    let size = f.size();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);
    
    let script_name = &app.scripts[app.selected_index].name;
    let title = Paragraph::new(
        format!("Running: {}", script_name)
    )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Script Output")
                .border_style(
                    Style::default().fg(Color::Green)
                )
        );
    f.render_widget(title, chunks[0]);
    
    let output = Paragraph::new(app.output_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(Color::Green)
                )
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(output, chunks[1]);
    
    let footer = Paragraph::new(
        "Press any key to go back"
    )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(Color::Green)
                )
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
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
    if event::poll(std::time::Duration::from_millis(100))? {
        // In run_app, inside the input handling:

        if let Event::Key(key) = event::read()? {
            if app.viewing_output {
                // Calculate max scroll for bounds checking
                let lines: Vec<&str> = app.output_text
                    .lines()
                    .collect();
                let total = lines.len();
                
                // This is approximate, but good enough
                let visible = 20;  

                //  Calculate max scroll each time because the terminal might 
                //    resize.  In a production app, we'd cache this, but for 
                //    simplicity, just recalculate. 
                let max_scroll = total.saturating_sub(visible);
                
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.scroll_output_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.scroll_output_down(max_scroll);
                    }
                    _ => {
                        app.back_to_list();
                    }
                }
            }
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
                    if let Err(e) = app.run_selected_script(&mut terminal) {
                        app.output_text = format!("Error:\n{}", e);
                        app.viewing_output = true;
                    }
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

fn render_output_view(
    f: &mut ratatui::Frame,
    app: &App,
) {
    let size = f.size();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);
    
    // Title
    let script_name = &app.scripts[app.selected_index].name;
    let title = Paragraph::new(
        format!("Output: {}", script_name)
    )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Script Output")
                .border_style(
                    Style::default().fg(Color::Green)
                )
        );
    f.render_widget(title, chunks[0]);
    
    // Calculate how many lines we can show
    // -2 for the borders
    let visible_height = chunks[1].height as usize - 2;
    
    // Split output into lines
    let lines: Vec<&str> = app.output_text
        .lines()
        .collect();
    let total_lines = lines.len();
    
    // Calculate max scroll
    let max_scroll = total_lines
        .saturating_sub(visible_height);
    
    // Get the visible slice of lines
    let start = app.output_scroll;
    let end = (start + visible_height).min(total_lines);
    let visible_lines: Vec<&str> = lines[start..end]
        .to_vec();
    
    // Join back into a single string
    let display_text = visible_lines.join("\n");
    
    let output = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(Color::Green)
                )
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(output, chunks[1]);
    
    // Footer with scroll indicator
    let scroll_info = if total_lines > visible_height {
        format!(
            "↑/↓: Scroll | Lines {}-{} of {} | Any other key: Back",
            start + 1,
            end,
            total_lines
        )
    } else {
        "Press any key to go back".to_string()
    };
    
    let footer = Paragraph::new(scroll_info)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(Color::Green)
                )
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
}

fn run_selected_script(
    &mut self,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    let script = &self.scripts[self.selected_index];
    
    self.output_text = "Running script...\n\n\
        Please wait...".to_string();
    self.viewing_output = true;
    
    terminal.draw(|f| {
        render_output_view(f, self);
    })?;
    
    let output = Command::new(&script.path).output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);
    
    // Format based on success/failure
    self.output_text = if exit_code == 0 {
        format!(
            "✓ Script completed successfully\n\
             Exit code: 0\n\n\
             === OUTPUT ===\n{}\n\n\
             === ERRORS ===\n{}",
            if stdout.is_empty() { 
                "(no output)" 
            } else { 
                stdout.as_ref() 
            },
            if stderr.is_empty() { 
                "(none)" 
            } else { 
                stderr.as_ref() 
            }
        )
    } else {
        format!(
            "✗ Script failed\n\
             Exit code: {}\n\n\
             === OUTPUT ===\n{}\n\n\
             === ERRORS ===\n{}",
            exit_code,
            if stdout.is_empty() { 
                "(no output)" 
            } else { 
                stdout.as_ref() 
            },
            if stderr.is_empty() { 
                "(none)" 
            } else { 
                stderr.as_ref() 
            }
        )
    };
    
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
