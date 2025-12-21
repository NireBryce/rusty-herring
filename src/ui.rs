use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use crate::app::App;

pub fn render_list_view(
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
    
    let title = Paragraph::new(
        format!(
            "Script Runner - {} scripts",
            app.scripts.len()
        )
    )
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
            
            let name = format!("{} {}", prefix, script.name);
            
            let lines = if let Some(d) = &script.description {
                vec![name, format!("    {}", d)]
            } else {
                vec![name]
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
        "↑/↓: Navigate | Enter: Run | ?: Help | q: Quit"
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

pub fn render_output_view(
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
    
    let is_success = app.output_text.starts_with("✓");
    let color = if is_success {
        Color::Green
    } else if app.output_text.starts_with("✗") {
        Color::Red
    } else {
        Color::Yellow
    };
    
    let script_name = &app.scripts[app.selected_index].name;
    let title = Paragraph::new(
        format!("Output: {}", script_name)
    )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Script Output")
                .border_style(Style::default().fg(color))
        );
    f.render_widget(title, chunks[0]);
    
    let visible_height = chunks[1].height as usize - 2;
    let lines: Vec<&str> = app.output_text
        .lines()
        .collect();
    let total = lines.len();
    
    let start = app.output_scroll;
    let end = (start + visible_height).min(total);
    let visible: Vec<&str> = lines[start..end].to_vec();
    
    let output = Paragraph::new(visible.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(output, chunks[1]);
    
    let footer_text = if total > visible_height {
        format!(
            "↑/↓: Scroll | Lines {}-{} of {} | Other: Back",
            start + 1,
            end,
            total
        )
    } else {
        "Press any key to go back".to_string()
    };
    
    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
}

pub fn render_help_view(f: &mut ratatui::Frame) {
    let size = f.size();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);
    
    let title = Paragraph::new("Keyboard Shortcuts")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(
                    Style::default().fg(Color::Yellow)
                )
        );
    f.render_widget(title, chunks[0]);
    
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
  Navigation uses vim keys (j/k) or arrows";
    
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
