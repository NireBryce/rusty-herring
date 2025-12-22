use std::env;
use std::io;
use std::process::Command;

use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    execute,
};

use rusty_herring::{App, scan_directory, ui};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn run_selected_script(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    let script = &app.scripts[app.selected_index];

    app.output_text = "Running script...\n\nPlease wait...".to_string();
    app.viewing_output = true;

    terminal.draw(|f| {
        ui::render_output_view(f, app);
    })?;

    let output = Command::new(&script.path).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);

    app.output_text = if code == 0 {
        format!(
            "✓ Script completed successfully\n\
             Exit code: 0\n\n\
             === OUTPUT ===\n{}\n\n\
             === ERRORS ===\n{}",
            if stdout.is_empty() { "(no output)" } else { stdout.as_ref() },
            if stderr.is_empty() { "(none)" } else { stderr.as_ref() }
        )
    } else {
        format!(
            "✗ Script failed\n\
             Exit code: {}\n\n\
             === OUTPUT ===\n{}\n\n\
             === ERRORS ===\n{}",
            code,
            if stdout.is_empty() { "(no output)" } else { stdout.as_ref() },
            if stderr.is_empty() { "(none)" } else { stderr.as_ref() }
        )
    };

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
                            if let Err(e) = run_selected_script(&mut app, terminal) {
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
