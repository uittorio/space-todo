use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};

use crate::{dashboard::state::Model, logs::Log};

pub fn render_logs(frame: &mut Frame, model: &Model) {
    let [top, middle] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Fill(1)])
        .areas(frame.area());

    let block = Block::bordered().title("Commands");

    let paragraph = Paragraph::new(
        "(-> <-) to focus view, (a) to add todos or boards | (e) to edit | (spacebar) to toggle todos | (d) delete | (q) quit",
    )
    .block(block)
    .alignment(Alignment::Left);

    frame.render_widget(paragraph, top);

    let middle_block = Block::bordered()
        .title("Logs")
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::new().bold().green());
    let middle_area = middle_block.inner(middle);

    frame.render_widget(middle_block, middle);

    let text = Text::from(
        model
            .logger
            .logs()
            .rev()
            .map(|l| match l {
                Log::Info(info) => Line::from(info.to_string()),
                Log::Error(error) => Line::from(error.to_string()).style(Color::Red),
            })
            .collect::<Vec<Line>>(),
    );
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, middle_area);
}
