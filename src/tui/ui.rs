use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use super::app::{App, FieldKind, FilterFocus, PipelineRow, TreeRow, TAB_NAMES};
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
    if app.active_tab == 1 {
        draw_filters_tab(f, app, area);
        return;
    }
    if app.active_tab == 2 {
        draw_pipeline_tab(f, app, area);
        return;
    }

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

fn draw_filters_tab(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filters ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let fs = &app.filter_state;

    // If no BIDS dir set
    if app.form.bids_dir.trim().is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Set BIDS directory in Input/Output tab first",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, inner);
        return;
    }

    // If scanned but no tree
    let has_runs = fs.tree.as_ref().is_some_and(|t| !t.subjects.is_empty());
    if !has_runs {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No QSM-compatible runs found in BIDS directory",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, inner);
        return;
    }

    let tree = fs.tree.as_ref().unwrap();

    // Build lines to render
    let mut lines: Vec<Line> = Vec::new();

    // Pattern input
    let pattern_focused = fs.focus == FilterFocus::Pattern;
    let pattern_label = Span::styled(
        "  Pattern: ",
        if pattern_focused { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) }
        else { Style::default().fg(Color::White) },
    );
    let pattern_val = if fs.pattern.is_empty() && !fs.pattern_editing {
        Span::styled("(enter glob to filter)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&fs.pattern, Style::default().fg(Color::Cyan))
    };
    lines.push(Line::from(vec![pattern_label, pattern_val]));
    lines.push(Line::from(""));

    // Tree rows
    let visible = fs.visible_rows();
    for (i, row) in visible.iter().enumerate() {
        let focused = fs.focus == FilterFocus::TreeNode(i);
        let line = match row {
            TreeRow::Subject(si) => {
                let sub = &tree.subjects[*si];
                let collapsed = fs.collapsed.contains(&format!("sub-{}", sub.name));
                let arrow = if collapsed { "▶" } else { "▼" };
                let sel = sub.selected_runs();
                let total = sub.total_runs();
                let sel_info = if sel == total { "all selected".to_string() } else { format!("{}/{} selected", sel, total) };
                let style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                };
                Line::from(Span::styled(
                    format!("  {} sub-{} ({} run{}, {})", arrow, sub.name, total, if total == 1 { "" } else { "s" }, sel_info),
                    style,
                ))
            }
            TreeRow::Session(si, sei) => {
                let sub = &tree.subjects[*si];
                let ses = &sub.sessions[*sei];
                let collapsed = fs.collapsed.contains(&format!("sub-{}/ses-{}", sub.name, ses.name));
                let arrow = if collapsed { "▶" } else { "▼" };
                let style = if focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(
                    format!("    {} ses-{}", arrow, ses.name),
                    style,
                ))
            }
            TreeRow::Run { sub, ses, run } => {
                let leaf = match ses {
                    Some(sei) => &tree.subjects[*sub].sessions[*sei].runs[*run],
                    None => &tree.subjects[*sub].runs[*run],
                };
                let indent = if ses.is_some() { "      " } else { "    " };
                let (marker, color) = if leaf.selected {
                    ("[x]", Color::Green)
                } else {
                    ("[ ]", Color::Gray)
                };
                let style = if focused {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };
                Line::from(Span::styled(
                    format!("{}{} {}", indent, marker, leaf.display),
                    style,
                ))
            }
        };
        lines.push(line);
    }

    // Blank line + Num Echoes
    lines.push(Line::from(""));
    let ne_focused = fs.focus == FilterFocus::NumEchoes;
    let ne_label = Span::styled(
        "  Num Echoes: ",
        if ne_focused { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) }
        else { Style::default().fg(Color::White) },
    );
    let ne_val = if fs.num_echoes.is_empty() && !fs.num_echoes_editing {
        Span::styled("(all)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&fs.num_echoes, Style::default().fg(Color::Cyan))
    };
    lines.push(Line::from(vec![ne_label, ne_val]));

    // Summary line
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {} run(s), {} selected", tree.total_runs(), tree.selected_runs()),
        Style::default().fg(Color::DarkGray),
    )));

    // Handle scrolling
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let scroll = if total_lines > visible_height {
        fs.scroll_offset.min(total_lines.saturating_sub(visible_height))
    } else {
        0
    };

    let para = Paragraph::new(lines).scroll((scroll as u16, 0));
    f.render_widget(para, inner);

    // Set cursor position if editing
    if fs.pattern_editing {
        let x = inner.x + 11 + fs.pattern_cursor as u16; // "  Pattern: " = 11 chars
        f.set_cursor_position((x, inner.y));
    } else if fs.num_echoes_editing {
        // Find the line offset for num_echoes
        let ne_line = visible.len() + 3; // pattern + blank + tree rows + blank
        if ne_line >= scroll && ne_line < scroll + visible_height {
            let y = inner.y + (ne_line - scroll) as u16;
            let x = inner.x + 14 + fs.num_echoes_cursor as u16; // "  Num Echoes: " = 14
            f.set_cursor_position((x, y));
        }
    }
}

fn draw_pipeline_tab(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Pipeline ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split into form area + help text area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(2)])
        .split(inner);
    let form_area = chunks[0];
    let help_area = chunks[1];

    let ps = &app.pipeline_state;
    let rows = ps.visible_rows();
    let focusable = ps.focusable_rows();

    let mut lines: Vec<Line> = Vec::new();
    let mut focused_help: Option<String> = None;

    let mut focusable_idx = 0;
    for (i, row) in rows.iter().enumerate() {
        let is_focusable = focusable.contains(&i);
        let focused = is_focusable && focusable_idx == ps.focus;
        if is_focusable {
            focusable_idx += 1;
        }

        let line = match row {
            PipelineRow::AlgoSelect { label, field, options, help } => {
                let selected = ps.get_select(field);
                let val = options.get(selected).unwrap_or(&"?");
                if focused {
                    focused_help = help.get(selected).map(|s| s.to_string());
                }
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                if focused {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", format!("{}:", label)), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(*val, Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", format!("{}:", label)), label_style),
                        Span::styled(*val, Style::default().fg(Color::Gray)),
                    ])
                }
            }
            PipelineRow::Param { label, field, help } => {
                let val = ps.get_param(field);
                if focused {
                    focused_help = Some(help.to_string());
                }
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let val_style = if focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let display_val = if val.is_empty() && !(focused && ps.editing) {
                    Span::styled("(default)", Style::default().fg(Color::DarkGray))
                } else {
                    Span::styled(val, val_style)
                };
                Line::from(vec![
                    Span::styled(format!("  {:22}", format!("{}:", label)), label_style),
                    display_val,
                ])
            }
            PipelineRow::Toggle { label, field, help } => {
                let checked = ps.get_toggle(field);
                if focused {
                    focused_help = Some(help.to_string());
                }
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let (marker, color) = if checked {
                    ("[x]", Color::Green)
                } else {
                    ("[ ]", Color::Gray)
                };
                Line::from(vec![
                    Span::styled(format!("  {:22}", format!("{}:", label)), label_style),
                    Span::styled(marker, Style::default().fg(color)),
                ])
            }
            PipelineRow::Separator => Line::from(""),
        };
        lines.push(line);
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, form_area);

    // Render help text
    if let Some(help) = focused_help {
        let help_para = Paragraph::new(Line::from(Span::styled(
            format!("  {}", help),
            Style::default().fg(Color::DarkGray),
        ))).wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(help_para, help_area);
    }

    // Set cursor if editing a param
    if ps.editing {
        let focusable_idx = ps.focus;
        if let Some(&row_idx) = focusable.get(focusable_idx) {
            let y = form_area.y + row_idx as u16;
            let label_width = 24;
            let x = form_area.x + label_width + ps.cursor as u16;
            if y < form_area.y + form_area.height {
                f.set_cursor_position((x, y));
            }
        }
    }
}

fn draw_command_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let cmd = command::build_command_string(app);
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
        for tab in 0..4 {
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
        app.active_tab = 2; // Pipeline tab
        let _ = render_app(&app);
        // Change algorithm
        app.pipeline_state.qsm_algorithm = 3; // TGV
        let _ = render_app(&app);
    }

    #[test]
    fn test_draw_execution_tab_with_flags() {
        let mut app = App::new();
        app.active_tab = 3;
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
