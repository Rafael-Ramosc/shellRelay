use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Clone, Copy)]
pub struct InstructionItem<'a> {
    pub label: &'a str,
    pub key: &'a str,
}

pub fn render_instructions(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    items: &[InstructionItem<'_>],
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let content_width = instruction_text_width(items);
    let total_width = area.width as usize;
    let side_len = total_width.saturating_sub(content_width + 2) / 2;
    let side = "\u{2500}".repeat(side_len);

    let mut spans: Vec<Span<'_>> = Vec::new();
    if !side.is_empty() {
        spans.push(Span::styled(side.clone(), Style::default().fg(Color::DarkGray)));
        spans.push(Span::raw(" "));
    }

    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(item.label, Style::default().fg(Color::Gray)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("<{}>", item.key),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if !side.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(side, Style::default().fg(Color::DarkGray)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn instruction_text_width(items: &[InstructionItem<'_>]) -> usize {
    let mut width = 0;
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            width += 2;
        }
        width += item.label.chars().count();
        width += 1;
        width += item.key.chars().count() + 2;
    }
    width
}
