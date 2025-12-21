use std::io;
use std::process::Command;
use ratatui::{backend::CrosstermBackend, Terminal};
use crate::script::Script;
use crate::ui;

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
        if self.selected_index < 
           self.scripts.len().saturating_sub(1) {
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
    
    pub fn run_selected_script(
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
    
    pub fn back_to_list(&mut self) {
        self.viewing_output = false;
        self.output_text.clear();
        self.output_scroll = 0;
    }
}
