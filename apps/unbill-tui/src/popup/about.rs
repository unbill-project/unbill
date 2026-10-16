use std::cell::Cell;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
};

use super::{PopupOutcome, PopupView, render_popup_base};

#[derive(Default)]
pub struct AboutPopup {
    scroll: Cell<u16>,
    max_scroll: Cell<u16>,
    page_height: Cell<u16>,
}

fn paragraphs() -> Paragraph<'static> {
    let about = unbill_about::content();
    let heading = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut lines = vec![
        Line::styled(about.name.as_str(), heading),
        Line::from(about.tagline.as_str()),
        Line::default(),
        Line::from(about.summary.as_str()),
        Line::default(),
        Line::styled(
            about.principles.join(" · "),
            Style::default().fg(Color::Cyan),
        ),
    ];
    for section in &about.sections {
        lines.push(Line::default());
        lines.push(Line::styled(section.title.as_str(), heading));
        for paragraph in &section.paragraphs {
            lines.push(Line::default());
            lines.push(Line::from(paragraph.as_str()));
        }
        if let Some(link) = &section.link {
            lines.push(Line::default());
            lines.push(Line::from(link.label.as_str()));
            lines.push(Line::styled(
                link.url.as_str(),
                Style::default().fg(Color::Cyan),
            ));
        }
    }
    Paragraph::new(lines).wrap(Wrap { trim: false })
}

// sirno:witness:about-page:begin
impl PopupView for AboutPopup {
    fn title(&self) -> &str {
        &unbill_about::content().title
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());
        let [body, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas::<2>(inner);
        let paragraph = paragraphs();
        let max_scroll = paragraph
            .line_count(body.width)
            .saturating_sub(usize::from(body.height));
        self.max_scroll
            .set(u16::try_from(max_scroll).unwrap_or(u16::MAX));
        self.scroll
            .set(self.scroll.get().min(self.max_scroll.get()));
        self.page_height.set(body.height.max(1));
        frame.render_widget(paragraph.scroll((self.scroll.get(), 0)), body);
        frame.render_widget(
            Paragraph::new("[j/k, PgUp/PgDn] scroll  [Home/End] first/last\n[s] copy repository  [c] copy contributors  [Esc] close")
                .wrap(Wrap { trim: false }).style(Style::default().fg(Color::DarkGray)),
            footer,
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        let current = self.scroll.get();
        match key.code {
            KeyCode::Esc => return PopupOutcome::Cancelled,
            KeyCode::Down | KeyCode::Char('j') => self.scroll.set(current.saturating_add(1)),
            KeyCode::Up | KeyCode::Char('k') => self.scroll.set(current.saturating_sub(1)),
            KeyCode::PageDown => self
                .scroll
                .set(current.saturating_add(self.page_height.get())),
            KeyCode::PageUp => self
                .scroll
                .set(current.saturating_sub(self.page_height.get())),
            KeyCode::Home | KeyCode::Char('g') => self.scroll.set(0),
            KeyCode::End | KeyCode::Char('G') => self.scroll.set(self.max_scroll.get()),
            KeyCode::Char('s') | KeyCode::Char('c') => {
                let id = if key.code == KeyCode::Char('s') {
                    "support"
                } else {
                    "contributors"
                };
                if let Some(link) = unbill_about::link(id) {
                    return PopupOutcome::CopyText(link.url.clone());
                }
            }
            _ => {}
        }
        self.scroll
            .set(self.scroll.get().min(self.max_scroll.get()));
        PopupOutcome::Pending
    }
}
// sirno:witness:about-page:end

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn long_content_can_reach_the_end_and_scroll_stays_in_bounds_after_resize() {
        let mut popup = AboutPopup::default();
        let mut terminal = Terminal::new(TestBackend::new(44, 16)).unwrap();
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .unwrap();
        assert!(popup.max_scroll.get() > 0);
        popup.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains(&unbill_about::link("support").unwrap().label));
        popup.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(popup.scroll.get(), popup.max_scroll.get());
        let mut large = Terminal::new(TestBackend::new(120, 80)).unwrap();
        large
            .draw(|frame| popup.render(frame, frame.area()))
            .unwrap();
        assert_eq!(popup.scroll.get(), 0);
        assert!(matches!(
            popup.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            PopupOutcome::Cancelled
        ));
    }

    #[test]
    fn copy_actions_use_the_shared_destinations() {
        let mut popup = AboutPopup::default();
        for (key, id) in [('s', "support"), ('c', "contributors")] {
            let outcome = popup.handle_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
            let PopupOutcome::CopyText(url) = outcome else {
                panic!("expected a clipboard action");
            };
            assert_eq!(url, unbill_about::link(id).unwrap().url);
        }
    }
}
