use std::collections::VecDeque;

use ratatui::{
    Frame,
    layout::{Constraint, Offset, Rect, Size},
    style::{Color, Style},
    widgets::{Cell, Clear, HighlightSpacing, Row, Table, TableState},
};
use ratatui_textarea::TextArea;

use crate::dashboard::state::{Model, View};

pub fn render_boards(frame: &mut Frame, area: Rect, textarea: &TextArea, model: &Model) {
    let boards_empty = model.boards.is_empty();

    let mut rows = if boards_empty {
        VecDeque::from_iter(vec![Row::new([Cell::from(
            "Press enter to add a new board!",
        )
        .style(Style::new().italic())])])
    } else {
        model
            .boards
            .iter()
            .map(|t| Row::new([Cell::from(format!("({}) {}", t.id.to_string(), t.name))]))
            .collect::<VecDeque<_>>()
    };

    let selected_board_index = model
        .current_board_id
        .as_ref()
        .map(|current_board| model.boards.iter().position(|b| b.id == *current_board))
        .flatten();
    let mut state = TableState::default().with_selected(selected_board_index);

    let highlight_style = if let View::Boards = model.current_view {
        Style::new().reversed()
    } else {
        Style::new().reversed().bg(Color::White).dark_gray()
    };

    let is_creating_new_board = match model.current_view {
        View::Boards => model.is_edit_mode && model.current_board_id.is_none(),
        View::Todos => false,
        View::Logs => false,
    };

    if is_creating_new_board && !boards_empty {
        rows.push_front(Row::new([Cell::from("Placeholder for textarea")]));
    }

    let mut table = Table::new(rows, [Constraint::Fill(1)])
        .row_highlight_style(highlight_style)
        .highlight_spacing(HighlightSpacing::Always);

    if let View::Boards = model.current_view {
        table = table.highlight_symbol(">>");
    } else {
        table = table.highlight_symbol("  ");
    };

    frame.render_stateful_widget(table, area, &mut state);

    match model.current_view {
        View::Boards => {
            if model.is_edit_mode {
                let text_area_index = selected_board_index.unwrap_or(0);
                let line = area
                    .offset(Offset::new(2, text_area_index as i32))
                    .resize(Size::new(area.width - 2, 1));
                frame.render_widget(Clear, line);
                frame.render_widget(textarea, line);
            }
        }
        View::Todos => {}
        View::Logs => {}
    }
}
