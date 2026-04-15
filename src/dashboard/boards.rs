use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Cell, Row, Table, TableState},
    Frame,
};

use crate::dashboard::state::{Model, View};

pub fn render_boards(frame: &mut Frame, area: Rect, model: &mut Model) {
    let selected_board_index = model
        .current_board_id
        .as_ref()
        .map(|current_board| model.boards.iter().position(|b| b.id == *current_board))
        .flatten();
    let mut state = TableState::default().with_selected(selected_board_index);

    let rows = model
        .boards
        .iter()
        .map(|t| Row::new([Cell::from(format!("({}) {}", t.id.to_string(), t.name))]));

    let style = if let View::Boards = model.current_view {
        Style::new().bold()
    } else {
        Style::new()
    };

    let header = Row::new(vec![Cell::from("Boards")])
        .style(style)
        .bottom_margin(1);

    let highlight_style = if let View::Boards = model.current_view {
        Style::new().reversed()
    } else {
        Style::new().reversed().bg(Color::White).dark_gray()
    };

    let mut table = Table::new(rows, [Constraint::Fill(1)])
        .header(header)
        .row_highlight_style(highlight_style);

    if let View::Boards = model.current_view {
        table = table.highlight_symbol(">>");
    } else {
        table = table.highlight_symbol("  ");
    };

    frame.render_stateful_widget(table, area, &mut state);
}
