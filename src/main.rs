use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use ratatui::{
    backend::CrosstermBackend,
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

mod ui;

#[derive(Debug)]
struct Script {
    path: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
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
    
    fn next(&mut self) {
        if self.selected_index < 
           self.scripts.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }
    
    fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    fn quit(&mut self) {
        self.should_quit = true;
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
    
    fn show_help(&mut self) {
        self.showing_help = true;
    }
    
    fn hide_help(&mut self) {
        self.showing_help = false;
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
            ui::render_output_view(f, self);
        })?;
        
        let output = Command::new(&script.path).output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);
        
        self.output_text = if code == 0 {
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
                code,
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
    
    fn back_to_list(&mut self) {
        self.viewing_output = false;
        self.output_text.clear();
        self.output_scroll = 0;
    }
}


struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn extract_description(
    path: &str
) -> Result<Option<String>, io::Error> {
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

fn scan_directory(
    directory: &str
) -> Result<Vec<Script>, io::Error> {
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

            let path_str = path
                .to_str()
                .unwrap_or("")
                .to_string();

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

            let path_str = path
                .to_str()
                .unwrap_or("")
                .to_string();

            let description = extract_description(&path_str)
                .unwrap_or(None);

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
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|f| {
            if app.showing_help {
                ui::render_help_view(f);
            } else if app.viewing_output {
                ui::render_output_view(f, &app);
            } else {
                ui::render_list_view(f, &app);
            }
        })?;
        
        if event::poll(
            std::time::Duration::from_millis(100)
        )? {
            if let Event::Key(key) = event::read()? {
                if app.showing_help {
                    app.hide_help();
                } else if app.viewing_output {
                    let lines: Vec<&str> = app.output_text
                        .lines()
                        .collect();
                    let total = lines.len();
                    let visible = 20;
                    let max = total.saturating_sub(visible);
                    
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scroll_output_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.scroll_output_down(max);
                        }
                        _ => {
                            app.back_to_list();
                        }
                    }
                } else {
                    match key.code {
                        KeyCode::Char('?') => {
                            app.show_help();
                        }
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
                            if let Err(e) = 
                                app.run_selected_script(terminal) 
                            {
                                app.output_text = format!(
                                    "✗ Error running script:\n{}",
                                    e
                                );
                                app.viewing_output = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if app.should_quit {
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
        println!(
            "No executable scripts in {}",
            directory
        );
        return Ok(());
    }
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let _guard = TerminalGuard;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let app = App::new(scripts);
    run_app(&mut terminal, app)?;
    
    Ok(())
}
