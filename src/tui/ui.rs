use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
    Frame,
};

use super::app::{App, FieldKind, FilterFocus, PipelineRow, TreeRow, TAB_NAMES};
use super::command;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Compute command preview height dynamically
    let cmd = command::build_command_string(app);
    let available_width = f.area().width.saturating_sub(4) as usize; // borders + padding
    let cmd_lines = if available_width > 0 {
        (cmd.len() / available_width + 1).max(1).min(6)
    } else {
        1
    };
    let preview_height = cmd_lines as u16 + 2; // +2 for borders

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),             // tab bar
            Constraint::Min(8),               // form
            Constraint::Length(preview_height), // command preview (dynamic)
            Constraint::Length(1),             // help bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_form(f, app, chunks[1]);
    draw_command_preview_with(f, &cmd, chunks[2]);
    draw_help_bar(f, app, chunks[3]);
}

/// Render a scrollable paragraph with a scrollbar when content exceeds visible height.
/// Updates `scroll_offset` in place so it persists between frames.
fn render_scrollable(
    f: &mut Frame,
    area: Rect,
    lines: Vec<Line<'_>>,
    scroll_offset: &mut usize,
    focused_line: Option<usize>,
) {
    let visible_height = area.height as usize;
    let total_lines = lines.len();

    if total_lines <= visible_height {
        *scroll_offset = 0;
        let para = Paragraph::new(lines);
        f.render_widget(para, area);
        return;
    }

    // Only scroll when focused line goes beyond the visible edges.
    // Otherwise the viewport stays put — this means the cursor can
    // freely move within the visible area without the view jumping.
    if let Some(fl) = focused_line {
        if fl < *scroll_offset {
            *scroll_offset = fl;
        } else if fl >= *scroll_offset + visible_height {
            *scroll_offset = fl - visible_height + 1;
        }
    }
    *scroll_offset = (*scroll_offset).min(total_lines.saturating_sub(visible_height));

    let scroll = *scroll_offset;
    let para = Paragraph::new(lines).scroll((scroll as u16, 0));
    f.render_widget(para, area);

    // Render scrollbar
    let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
        .position(scroll);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
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

fn draw_form(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
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

    // Split into scrollable form area + help text area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(2)])
        .split(inner);
    let form_area = chunks[0];
    let help_area = chunks[1];

    let fields = &app.tab_fields[app.active_tab];

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let focused = i == app.active_field;
        let editing = focused && app.editing;

        let line = match &field.kind {
            FieldKind::Text => {
                let value = app.get_text_value(app.active_tab, i).to_string();
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let val_style = if focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::Gray) };
                let display_val = if value.is_empty() && !editing {
                    Span::styled("(empty)", Style::default().fg(Color::DarkGray))
                } else {
                    Span::styled(value, val_style)
                };
                Line::from(vec![
                    Span::styled(format!("  {:22}", format!("{}:", field.label)), label_style),
                    display_val,
                ])
            }
            FieldKind::Select { options } => {
                let selected = app.get_select_value(app.active_tab, i);
                let val = options.get(selected).unwrap_or(&"?");
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                if focused {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", format!("{}:", field.label)), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(*val, Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", format!("{}:", field.label)), label_style),
                        Span::styled(*val, Style::default().fg(Color::Gray)),
                    ])
                }
            }
            FieldKind::Checkbox => {
                let checked = app.get_checkbox_value(app.active_tab, i);
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let (marker, color) = if checked { ("[x]", Color::Green) } else { ("[ ]", Color::Gray) };
                Line::from(vec![
                    Span::styled(format!("  {:22}", format!("{}:", field.label)), label_style),
                    Span::styled(marker, Style::default().fg(color)),
                ])
            }
        };
        lines.push(line);
    }

    render_scrollable(f, form_area, lines, &mut app.form_scroll_offset, Some(app.active_field));

    // Help text for focused field
    if app.active_field < fields.len() {
        let help = fields[app.active_field].help;
        if !help.is_empty() {
            let help_para = Paragraph::new(Line::from(Span::styled(
                format!("  {}", help),
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(help_para, help_area);
        }
    }

    // Set cursor if editing
    if app.editing {
        let scroll = app.form_scroll_offset;
        if app.active_field >= scroll && app.active_field < scroll + form_area.height as usize {
            let y = form_area.y + (app.active_field - scroll) as u16;
            let x = form_area.x + 24 + app.cursor_pos as u16;
            f.set_cursor_position((x, y));
        }
    }
}

fn draw_filters_tab(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filters ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // If no BIDS dir set
    if app.form.bids_dir.trim().is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Set BIDS directory in Input/Output tab first",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, inner);
        return;
    }

    {
    let fs = &app.filter_state;

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
    }

    let tree = app.filter_state.tree.as_ref().unwrap();

    // Build lines to render
    let mut lines: Vec<Line> = Vec::new();

    // Pattern input
    let pattern_focused = app.filter_state.focus == FilterFocus::Pattern;
    let pattern_label = Span::styled(
        "  Pattern: ",
        if pattern_focused { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) }
        else { Style::default().fg(Color::White) },
    );
    let pattern_val = if app.filter_state.pattern.is_empty() && !app.filter_state.pattern_editing {
        Span::styled("(enter glob to filter)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.filter_state.pattern, Style::default().fg(Color::Cyan))
    };
    lines.push(Line::from(vec![pattern_label, pattern_val]));
    lines.push(Line::from(""));

    // Tree rows
    let visible = app.filter_state.visible_rows();
    for (i, row) in visible.iter().enumerate() {
        let focused = app.filter_state.focus == FilterFocus::TreeNode(i);
        let line = match row {
            TreeRow::Subject(si) => {
                let sub = &tree.subjects[*si];
                let collapsed = app.filter_state.collapsed.contains(&format!("sub-{}", sub.name));
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
                let collapsed = app.filter_state.collapsed.contains(&format!("sub-{}/ses-{}", sub.name, ses.name));
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
    let ne_focused = app.filter_state.focus == FilterFocus::NumEchoes;
    let ne_label = Span::styled(
        "  Num Echoes: ",
        if ne_focused { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) }
        else { Style::default().fg(Color::White) },
    );
    let ne_val = if app.filter_state.num_echoes.is_empty() && !app.filter_state.num_echoes_editing {
        Span::styled("(all)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.filter_state.num_echoes, Style::default().fg(Color::Cyan))
    };
    lines.push(Line::from(vec![ne_label, ne_val]));

    // Summary line
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {} run(s), {} selected", tree.total_runs(), tree.selected_runs()),
        Style::default().fg(Color::DarkGray),
    )));

    // Capture values needed after mutable borrow
    let focus = app.filter_state.focus;
    let pattern_editing = app.filter_state.pattern_editing;
    let pattern_cursor = app.filter_state.pattern_cursor;
    let num_echoes_editing = app.filter_state.num_echoes_editing;
    let num_echoes_cursor = app.filter_state.num_echoes_cursor;
    let visible_len = visible.len();

    // Determine focused line for auto-scroll
    let focused_line = match focus {
        FilterFocus::Pattern => Some(0),
        FilterFocus::TreeNode(i) => Some(i + 2), // pattern + blank + tree index
        FilterFocus::NumEchoes => Some(visible_len + 3), // pattern + blank + tree + blank
    };

    render_scrollable(f, inner, lines, &mut app.filter_state.scroll_offset, focused_line);
    let scroll = app.filter_state.scroll_offset;

    // Set cursor position if editing
    if pattern_editing {
        let x = inner.x + 11 + pattern_cursor as u16;
        if scroll == 0 {
            f.set_cursor_position((x, inner.y));
        }
    } else if num_echoes_editing {
        let ne_line = visible_len + 3;
        if ne_line >= scroll && ne_line < scroll + inner.height as usize {
            let y = inner.y + (ne_line - scroll) as u16;
            let x = inner.x + 14 + num_echoes_cursor as u16;
            f.set_cursor_position((x, y));
        }
    }
}

fn draw_pipeline_tab(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
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

    // Build lines and collect state from pipeline_state before mutable borrow
    let rows = app.pipeline_state.visible_rows();
    let focusable = app.pipeline_state.focusable_rows();
    let ps_focus = app.pipeline_state.focus;
    let ps_editing = app.pipeline_state.editing;
    let ps_cursor = app.pipeline_state.cursor;

    let mut lines: Vec<Line> = Vec::new();
    let mut focused_help: Option<String> = None;

    let mut focusable_idx = 0;
    for (i, row) in rows.iter().enumerate() {
        let is_focusable = focusable.contains(&i);
        let focused = is_focusable && focusable_idx == ps_focus;
        if is_focusable {
            focusable_idx += 1;
        }

        let line = match row {
            PipelineRow::AlgoSelect { label, field, options, help } => {
                let selected = app.pipeline_state.get_select(field);
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
                let val = app.pipeline_state.get_param(field).to_string();
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
                let display_val = if val.is_empty() && !(focused && ps_editing) {
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
                let checked = app.pipeline_state.get_toggle(field);
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
            PipelineRow::MaskSectionHeader { section } => {
                Line::from(Span::styled(
                    format!("  ── Mask {} ──", section + 1),
                    Style::default().fg(Color::DarkGray),
                ))
            }
            PipelineRow::MaskOrSeparator => {
                Line::from(Span::styled(
                    "  ── COMBINED WITH ──",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                ))
            }
            PipelineRow::MaskOpInput { section } => {
                let input = &app.pipeline_state.mask_sections[*section].input;
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                if focused {
                    if focused_help.is_none() {
                        focused_help = Some("Masking input source (←/→ to change)".to_string());
                    }
                    Line::from(vec![
                        Span::styled(format!("  {:22}", "Input:"), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{}", input), Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", "Input:"), label_style),
                        Span::styled(format!("{}", input), Style::default().fg(Color::Gray)),
                    ])
                }
            }
            PipelineRow::MaskOpGenerator { section } => {
                let gen = &app.pipeline_state.mask_sections[*section].generator;
                let algo_name = match gen {
                    crate::pipeline::config::MaskOp::Threshold { .. } => "threshold",
                    crate::pipeline::config::MaskOp::Bet { .. } => "bet",
                    _ => "?",
                };
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                if focused {
                    if focused_help.is_none() {
                        focused_help = Some("Mask algorithm (←/→ to switch between threshold and BET)".to_string());
                    }
                    Line::from(vec![
                        Span::styled(format!("  {:22}", "Algorithm:"), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(algo_name, Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", "Algorithm:"), label_style),
                        Span::styled(algo_name, Style::default().fg(Color::Gray)),
                    ])
                }
            }
            PipelineRow::MaskOpGeneratorParam { section } => {
                let gen = &app.pipeline_state.mask_sections[*section].generator;
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let (label, val, help) = match gen {
                    crate::pipeline::config::MaskOp::Threshold { method, .. } => {
                        let method_name = match method {
                            crate::pipeline::config::MaskThresholdMethod::Otsu => "otsu",
                            crate::pipeline::config::MaskThresholdMethod::Fixed => "fixed",
                            crate::pipeline::config::MaskThresholdMethod::Percentile => "percentile",
                        };
                        ("Method:", method_name.to_string(), "Threshold method (←/→ to change)")
                    }
                    crate::pipeline::config::MaskOp::Bet { fractional_intensity } => {
                        ("Frac. Intensity:", format!("{:.2}", fractional_intensity), "BET fractional intensity 0.0-1.0, smaller = larger brain (←/→ to adjust)")
                    }
                    _ => ("?:", "?".to_string(), ""),
                };
                if focused {
                    if focused_help.is_none() {
                        focused_help = Some(help.to_string());
                    }
                    Line::from(vec![
                        Span::styled(format!("  {:22}", label), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(val, Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:22}", label), label_style),
                        Span::styled(val, Style::default().fg(Color::Gray)),
                    ])
                }
            }
            PipelineRow::MaskOpThresholdValue { section } => {
                let gen = &app.pipeline_state.mask_sections[*section].generator;
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let (label, val) = match gen {
                    crate::pipeline::config::MaskOp::Threshold { method: crate::pipeline::config::MaskThresholdMethod::Fixed, value } =>
                        ("Value:", value.map(|v| format!("{}", v)).unwrap_or("0.5".to_string())),
                    crate::pipeline::config::MaskOp::Threshold { method: crate::pipeline::config::MaskThresholdMethod::Percentile, value } =>
                        ("Percentile:", value.map(|v| format!("{}", v)).unwrap_or("75".to_string())),
                    _ => ("Value:", "?".to_string()),
                };
                let display_val = if app.pipeline_state.mask_threshold_editing && focused {
                    app.pipeline_state.mask_threshold_value_buf.clone()
                } else {
                    val
                };
                if focused {
                    if focused_help.is_none() {
                        focused_help = Some("Enter to edit value, Esc to cancel".to_string());
                    }
                }
                let val_style = if focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::Gray) };
                Line::from(vec![
                    Span::styled(format!("  {:22}", label), label_style),
                    Span::styled(display_val, val_style),
                ])
            }
            PipelineRow::MaskOpEntry { section, index } => {
                let op = &app.pipeline_state.mask_sections[*section].refinements[*index];
                let (op_type, op_val) = super::app::PipelineFormState::mask_op_label_value(op);
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
                if focused {
                    if focused_help.is_none() {
                        focused_help = Some(super::app::PipelineFormState::mask_op_help(op).to_string());
                    }
                    Line::from(vec![
                        Span::styled(format!("  {:3}", format!("{}.", index + 1)), label_style),
                        Span::styled(format!("{:19}", format!("{}:", op_type)), label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(op_val.clone(), val_style),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("  {:3}", format!("{}.", index + 1)), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:19}", format!("{}:", op_type)), label_style),
                        Span::styled(op_val, val_style),
                    ])
                }
            }
            PipelineRow::MaskOpAddStep { section } => {
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                if app.pipeline_state.mask_ops_adding && focused {
                    let available = app.pipeline_state.available_op_types(*section);
                    let type_name = available.get(app.pipeline_state.mask_ops_add_idx).copied().unwrap_or("?");
                    if focused_help.is_none() {
                        focused_help = Some("←/→ to select type, Enter to add, Esc to cancel".to_string());
                    }
                    Line::from(vec![
                        Span::styled("  +   ", label_style),
                        Span::styled("◀ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(type_name, Style::default().fg(Color::Cyan)),
                        Span::styled(" ▶", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    if focused && focused_help.is_none() {
                        focused_help = Some("Enter to add step, d to delete, Ctrl+↑/↓ to reorder".to_string());
                    }
                    Line::from(Span::styled("  + Add step...", label_style))
                }
            }
            PipelineRow::MaskOpAddSection => {
                let label_style = if focused {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                if focused && focused_help.is_none() {
                    focused_help = Some("Enter to add a new OR'd mask section".to_string());
                }
                Line::from(Span::styled("  + Add mask...", label_style))
            }
        };
        lines.push(line);
    }

    // Determine which line is focused for auto-scroll
    let focused_line = focusable.get(ps_focus).copied();

    render_scrollable(f, form_area, lines, &mut app.pipeline_state.scroll_offset, focused_line);
    let scroll = app.pipeline_state.scroll_offset;

    // Render help text
    if let Some(help) = focused_help {
        let help_para = Paragraph::new(Line::from(Span::styled(
            format!("  {}", help),
            Style::default().fg(Color::DarkGray),
        ))).wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(help_para, help_area);
    }

    // Set cursor if editing a param or threshold value
    if ps_editing || app.pipeline_state.mask_threshold_editing {
        if let Some(&row_idx) = focusable.get(ps_focus) {
            if row_idx >= scroll && row_idx < scroll + form_area.height as usize {
                let y = form_area.y + (row_idx - scroll) as u16;
                let label_width = 24;
                let x = form_area.x + label_width + ps_cursor as u16;
                f.set_cursor_position((x, y));
            }
        }
    }
}

fn draw_command_preview_with(f: &mut Frame, cmd: &str, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Command Preview ");
    let para = Paragraph::new(Line::from(Span::styled(
        cmd.to_string(),
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

    fn render_app(app: &mut App) -> Terminal<TestBackend> {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal
    }

    #[test]
    fn test_draw_default_app_no_panic() {
        let mut app = App::new();
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_all_tabs() {
        let mut app = App::new();
        for tab in 0..4 {
            app.active_tab = tab;
            app.active_field = 0;
            let _ = render_app(&mut app);
        }
    }

    #[test]
    fn test_draw_editing_mode() {
        let mut app = App::new();
        app.editing = true;
        app.form.bids_dir = "/some/path".to_string();
        app.cursor_pos = 5;
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_with_form_data() {
        let mut app = App::new();
        app.form.bids_dir = "/data/bids".to_string();
        app.form.output_dir = "/data/out".to_string();
        app.form.preset = 1;
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_algorithms_tab() {
        let mut app = App::new();
        app.active_tab = 2;
        app.active_field = 0;
        let _ = render_app(&mut app);
        // Move through fields
        app.active_field = 4;
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_parameters_tab() {
        let mut app = App::new();
        app.active_tab = 2; // Pipeline tab
        let _ = render_app(&mut app);
        // Change algorithm
        app.pipeline_state.qsm_algorithm = 3; // TGV
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_execution_tab_with_flags() {
        let mut app = App::new();
        app.active_tab = 3;
        app.form.do_swi = true;
        app.form.do_t2starmap = true;
        app.form.dry_run = true;
        app.form.debug = true;
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_non_focused_fields() {
        let mut app = App::new();
        app.active_tab = 0;
        app.active_field = 3; // Last field focused, others not
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_select_not_focused() {
        let mut app = App::new();
        app.active_tab = 2;
        app.active_field = 1; // field 0 (select) not focused
        let _ = render_app(&mut app);
    }

    #[test]
    fn test_draw_empty_text_not_editing() {
        let mut app = App::new();
        // All fields empty, not editing — shows "(empty)"
        app.active_tab = 0;
        app.active_field = 0;
        let _ = render_app(&mut app);
    }
}
