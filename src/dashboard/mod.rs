use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Paragraph},
};
use ratatui_textarea::TextArea;

use crate::dashboard::{
    boards::render_boards,
    state::{Model, View},
    todos::render_todos,
};

pub mod boards;
pub mod state;
pub mod todos;

pub fn render(frame: &mut Frame, textarea: &mut TextArea, model: &mut Model) {
    let [top, middle, bottom] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

    let block = Block::bordered().title("Commands");

    let paragraph = Paragraph::new(
        "(-> <-) to focus view, (a) to add todos or boards | (e) to edit | (spacebar) to toggle todos | (d) delete | (q) quit",
    )
    .block(block)
    .alignment(Alignment::Left);

    frame.render_widget(paragraph, top);

    let [left, right] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(50), Constraint::Fill(1)])
        .areas(middle);

    let mut left_block = Block::bordered()
        .title("Boards")
        .border_type(ratatui::widgets::BorderType::Rounded);
    let left_area = left_block.inner(left);

    let mut right_block = Block::bordered()
        .title("Todos")
        .border_type(ratatui::widgets::BorderType::Rounded);
    let right_area = right_block.inner(right);

    match model.current_view {
        View::Todos => right_block = right_block.border_style(Style::new().bold().cyan()),
        View::Boards => left_block = left_block.border_style(Style::new().bold().cyan()),
    }

    frame.render_widget(left_block, left);
    frame.render_widget(right_block, right);

    render_boards(frame, left_area, textarea, model);
    render_todos(frame, right_area, textarea, model);

    let empty = String::new();
    let paragraph = Paragraph::new(
        Span::default()
            .content(model.last_error.as_ref().unwrap_or(&empty).as_str())
            .style(Style::new().fg(Color::Red)),
    )
    .block(Block::bordered());
    frame.render_widget(paragraph, bottom);
}
