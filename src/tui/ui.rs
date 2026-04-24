use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use super::app::{App, FieldKind, TAB_NAMES};
use super::command;
use super::widgets;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // tab bar
            Constraint::Min(8),    // form
            Constraint::Length(4), // command preview
            Constraint::Length(1), // help bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_form(f, app, chunks[1]);
    draw_command_preview(f, app, chunks[2]);
    draw_help_bar(f, app, chunks[3]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let titles: Vec<Line> = TAB_NAMES
        .iter()
        .enumerate()
        .map(|(i, t)| Line::from(format!(" {}:{} ", i + 1, t)))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" QSMxT "))
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    f.render_widget(tabs, area);
}

fn draw_form(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", TAB_NAMES[app.active_tab]));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let fields = &app.tab_fields[app.active_tab];

    // Each field gets 2 lines of height; remaining space for help text
    let mut constraints: Vec<Constraint> = fields
        .iter()
        .map(|_| Constraint::Length(1))
        .collect();
    constraints.push(Constraint::Length(1)); // spacer
    constraints.push(Constraint::Min(0));    // help text area

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, field) in fields.iter().enumerate() {
        let focused = i == app.active_field;
        let editing = focused && app.editing;

        match &field.kind {
            FieldKind::Text => {
                let value = app.get_text_value(app.active_tab, i);
                let cursor = if editing { Some(app.cursor_pos) } else { None };
                widgets::render_text_input(f, rows[i], field.label, value, focused, cursor);
            }
            FieldKind::Select { options } => {
                let selected = app.get_select_value(app.active_tab, i);
                widgets::render_select(f, rows[i], field.label, options, selected, focused);
            }
            FieldKind::Checkbox => {
                let checked = app.get_checkbox_value(app.active_tab, i);
                widgets::render_checkbox(f, rows[i], field.label, checked, focused);
            }
        }
    }

    // Help text for focused field
    if app.active_field < fields.len() {
        let help = fields[app.active_field].help;
        if !help.is_empty() {
            let help_idx = fields.len() + 1; // after spacer
            if help_idx < rows.len() {
                let help_para = Paragraph::new(Line::from(Span::styled(
                    format!("  {}", help),
                    Style::default().fg(Color::DarkGray),
                )));
                f.render_widget(help_para, rows[help_idx]);
            }
        }
    }
}

fn draw_command_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let cmd = command::build_command_string(&app.form);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Command Preview ");
    let para = Paragraph::new(Line::from(Span::styled(
        cmd,
        Style::default().fg(Color::Green),
    )))
    .block(block)
    .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(para, area);
}

fn draw_help_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help = if app.editing {
        vec![
            Span::styled(" Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Cancel  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Confirm", Style::default().fg(Color::DarkGray)),
        ]
    } else {
        vec![
            Span::styled(" 1-5", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Tabs  ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2191}\u{2193}", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2190}\u{2192}/Space", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Change  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Edit  ", Style::default().fg(Color::DarkGray)),
            Span::styled("F5", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Run  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Quit", Style::default().fg(Color::DarkGray)),
        ]
    };

    f.render_widget(Paragraph::new(Line::from(help)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_app(app: &App) -> Terminal<TestBackend> {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal
    }

    #[test]
    fn test_draw_default_app_no_panic() {
        let app = App::new();
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_all_tabs() {
        let mut app = App::new();
        for tab in 0..5 {
            app.active_tab = tab;
            app.active_field = 0;
            let _ = render_app(&app);
        }
    }

    #[test]
    fn test_draw_editing_mode() {
        let mut app = App::new();
        app.editing = true;
        app.form.bids_dir = "/some/path".to_string();
        app.cursor_pos = 5;
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_with_form_data() {
        let mut app = App::new();
        app.form.bids_dir = "/data/bids".to_string();
        app.form.output_dir = "/data/out".to_string();
        app.form.preset = 1;
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_algorithms_tab() {
        let mut app = App::new();
        app.active_tab = 2;
        app.active_field = 0;
        let _ = render_app(&app);
        // Move through fields
        app.active_field = 4;
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_parameters_tab() {
        let mut app = App::new();
        app.active_tab = 3;
        // Checkbox focused
        app.active_field = 0;
        app.form.combine_phase = true;
        let _ = render_app(&app);
        // Text field focused
        app.active_field = 1;
        app.form.bet_fractional_intensity = "0.5".to_string();
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_execution_tab_with_flags() {
        let mut app = App::new();
        app.active_tab = 4;
        app.form.do_swi = true;
        app.form.do_t2starmap = true;
        app.form.dry_run = true;
        app.form.debug = true;
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_non_focused_fields() {
        let mut app = App::new();
        app.active_tab = 0;
        app.active_field = 3; // Last field focused, others not
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_select_not_focused() {
        let mut app = App::new();
        app.active_tab = 2;
        app.active_field = 1; // field 0 (select) not focused
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_empty_text_not_editing() {
        let mut app = App::new();
        // All fields empty, not editing — shows "(empty)"
        app.active_tab = 0;
        app.active_field = 0;
        let _ = render_app(&app);
    }
}
