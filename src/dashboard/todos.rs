use std::collections::VecDeque;

use ratatui::{
    Frame,
    layout::{Constraint, Offset, Rect, Size},
    style::Style,
    widgets::{Cell, Clear, HighlightSpacing, Row, Table, TableState},
};
use ratatui_textarea::TextArea;

use crate::dashboard::state::{Model, View};

pub fn render_todos(frame: &mut Frame, area: Rect, textarea: &TextArea, model: &mut Model) {
    let todos_empty = model.todos.is_empty();

    let mut rows = if todos_empty {
        VecDeque::from_iter(vec![Row::new([Cell::from(
            "Press enter to add a new todo!",
        )
        .style(Style::new().italic())])])
    } else {
        model
            .todos
            .iter()
            .map(|t| {
                let row = Row::new([Cell::from(t.name.to_string())]);
                if t.done {
                    row.style(Style::new().green())
                } else {
                    row
                }
            })
            .collect::<VecDeque<_>>()
    };

    let is_creating_new_todo = model.is_edit_mode && model.current_todo_index.is_none();
    if is_creating_new_todo && !model.todos.is_empty() {
        rows.push_front(Row::new([Cell::from("Placeholder for textarea")]));
    }

    let style = if let View::Todos = model.current_view {
        Style::new().bold()
    } else {
        Style::new()
    };

    let header = Row::new(vec![Cell::from("Todos")])
        .style(style)
        .bottom_margin(1);

    let table = Table::new(rows, [Constraint::Fill(1)])
        .header(header)
        .row_highlight_style(Style::new().reversed())
        .highlight_symbol(">>")
        .highlight_spacing(HighlightSpacing::Always);

    let mut table_state = TableState::new().with_selected(model.current_todo_index);
    frame.render_stateful_widget(table, area, &mut table_state);

    if model.is_edit_mode {
        let text_area_index = model.current_todo_index.unwrap_or(0);
        let line = area
            .offset(Offset::new(2, 1 + 1 + text_area_index as i32))
            .resize(Size::new(area.width - 2, 1));
        frame.render_widget(Clear, line);
        frame.render_widget(textarea, line);
    }
}
