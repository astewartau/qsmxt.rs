use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const LABEL_WIDTH: u16 = 24;

fn label_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn value_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn split_label_value(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(1)])
        .split(area);
    (chunks[0], chunks[1])
}

pub fn render_text_input(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    cursor: Option<usize>,
) {
    let (label_area, value_area) = split_label_value(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}: ", label),
            label_style(focused),
        ))),
        label_area,
    );

    let display = if value.is_empty() && cursor.is_none() {
        Span::styled("(empty)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(value, value_style(focused))
    };

    f.render_widget(Paragraph::new(Line::from(display)), value_area);

    if let Some(pos) = cursor {
        let x = value_area.x + pos as u16;
        let y = value_area.y;
        f.set_cursor_position((x, y));
    }
}

pub fn render_select(
    f: &mut Frame,
    area: Rect,
    label: &str,
    options: &[&str],
    selected: usize,
    focused: bool,
) {
    let (label_area, value_area) = split_label_value(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}: ", label),
            label_style(focused),
        ))),
        label_area,
    );

    let opt = options.get(selected).unwrap_or(&"?");
    let display = if focused {
        Line::from(vec![
            Span::styled("\u{25C0} ", Style::default().fg(Color::DarkGray)),
            Span::styled(*opt, value_style(true)),
            Span::styled(" \u{25B6}", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(Span::styled(*opt, value_style(false)))
    };

    f.render_widget(Paragraph::new(display), value_area);
}

pub fn render_checkbox(
    f: &mut Frame,
    area: Rect,
    label: &str,
    checked: bool,
    focused: bool,
) {
    let (label_area, value_area) = split_label_value(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}: ", label),
            label_style(focused),
        ))),
        label_area,
    );

    let (marker, color) = if checked {
        ("[x]", Color::Green)
    } else {
        ("[ ]", Color::Gray)
    };

    let style = if focused {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(marker, style))),
        value_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn test_frame(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_text_input_empty_no_cursor() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_text_input(f, f.area(), "Label", "", false, None);
        }).unwrap();
    }

    #[test]
    fn test_render_text_input_with_value() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_text_input(f, f.area(), "Path", "/data/bids", true, None);
        }).unwrap();
    }

    #[test]
    fn test_render_text_input_with_cursor() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_text_input(f, f.area(), "Path", "/data", true, Some(3));
        }).unwrap();
    }

    #[test]
    fn test_render_select_focused() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_select(f, f.area(), "Algorithm", &["rts", "tv", "tkd"], 1, true);
        }).unwrap();
    }

    #[test]
    fn test_render_select_not_focused() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_select(f, f.area(), "Algorithm", &["rts", "tv"], 0, false);
        }).unwrap();
    }

    #[test]
    fn test_render_select_out_of_range() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_select(f, f.area(), "X", &["a"], 99, false);
        }).unwrap();
    }

    #[test]
    fn test_render_checkbox_checked() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_checkbox(f, f.area(), "SWI", true, true);
        }).unwrap();
    }

    #[test]
    fn test_render_checkbox_unchecked() {
        let mut term = test_frame(80, 3);
        term.draw(|f| {
            render_checkbox(f, f.area(), "SWI", false, false);
        }).unwrap();
    }

    #[test]
    fn test_label_style_focused() {
        let s = label_style(true);
        assert_eq!(s.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_label_style_unfocused() {
        let s = label_style(false);
        assert_eq!(s.fg, Some(Color::White));
    }

    #[test]
    fn test_value_style_focused() {
        let s = value_style(true);
        assert_eq!(s.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_value_style_unfocused() {
        let s = value_style(false);
        assert_eq!(s.fg, Some(Color::Gray));
    }
}
