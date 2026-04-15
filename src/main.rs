mod module_bindings;
use crossterm::event::{Event, KeyCode};
use dotenv::dotenv;
use module_bindings::*;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Block,
};
use ratatui_textarea::TextArea;
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};
use std::{
    env,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use crate::dashboard::state::{AppEvent, Model, View, update};
use crate::dashboard::{boards::render_boards, todos::render_todos};
mod dashboard;

fn main() -> color_eyre::Result<()> {
    dotenv().ok();
    let (tx, rx): (Sender<AppEvent>, Receiver<AppEvent>) = mpsc::channel();
    let id_token = env::var("SPACETIMEDB_TOKEN").unwrap();

    // The URI of the SpacetimeDB instance hosting our chat module.
    let host: String =
        env::var("SPACETIMEDB_HOST").unwrap_or("https://maincloud.spacetimedb.com".to_string());

    // The module name we chose when we published our module.
    let db_name: String = env::var("SPACETIMEDB_DB_NAME").unwrap_or("space-todo-fn915".to_string());

    // Connect to the database
    let conn = DbConnection::builder()
        .with_database_name(db_name)
        .with_uri(host)
        .with_token(Some(id_token))
        .on_connect(|_, _, _| {
            println!("Connected to SpacetimeDB");
        })
        .on_connect_error(|_ctx, e| {
            eprintln!("Connection error: {:?}", e);
            std::process::exit(1);
        })
        .build()
        .expect("Failed to connect");

    conn.subscription_builder()
        .on_applied(|_ctx| {})
        .on_error(|_ctx, e| eprintln!("There was an error when subscring to the table: {e}"))
        .add_query(|q| q.from.todos())
        .add_query(|q| q.from.my_boards())
        .add_query(|q| q.from.my_user())
        .subscribe();

    conn.db.my_boards().on_insert({
        let tx = tx.clone();
        move |_, r| {
            tx.send(AppEvent::OnBoardAdded(r.clone())).ok();
        }
    });

    conn.db.todos().on_insert({
        let tx = tx.clone();
        move |_, r| {
            tx.send(AppEvent::OnTodoAdded(r.clone())).ok();
        }
    });

    conn.db.todos().on_update({
        let tx = tx.clone();
        move |_, _, n| {
            tx.send(AppEvent::OnTodoUpdated(n.clone())).ok();
        }
    });

    conn.db.my_boards().on_delete({
        let tx = tx.clone();
        move |_, r| {
            tx.send(AppEvent::OnBoardDeleted(r.clone())).ok();
        }
    });

    conn.db.todos().on_delete({
        let tx = tx.clone();
        move |_, r| {
            tx.send(AppEvent::OnTodoDeleted(r.clone())).ok();
        }
    });

    conn.db.my_user().on_update({
        let tx = tx.clone();
        move |_, _, n| {
            let current_board_id = if n.current_board == 0 {
                None
            } else {
                Some(n.current_board)
            };

            tx.send(AppEvent::OnCurrentBoardUpdated(current_board_id))
                .ok();
        }
    });

    conn.db.my_user().on_insert({
        let tx = tx.clone();
        move |_, r| {
            let current_board_id = if r.current_board == 0 {
                None
            } else {
                Some(r.current_board)
            };

            tx.send(AppEvent::OnCurrentBoardUpdated(current_board_id))
                .ok();
        }
    });

    conn.run_threaded();

    color_eyre::install()?;

    ratatui::run(|a| app(a, rx, &conn))?;

    conn.disconnect()?;
    Ok(())
}

fn app(
    terminal: &mut DefaultTerminal,
    receiver: Receiver<AppEvent>,
    conn: &DbConnection,
) -> std::io::Result<()> {
    let mut model = Model {
        boards: vec![],
        todos: vec![],
        current_board_id: None,
        current_view: View::Boards,
        current_todo_index: None,
        is_edit_mode: false,
    };

    let mut textarea = TextArea::new(vec![]);

    textarea.set_placeholder_text("Type to create a todo");
    textarea.set_style(Style::new().on_black());

    loop {
        for event in receiver.try_iter() {
            update(&mut model, event);
        }
        terminal.draw(|frame| render(frame, &textarea, &mut model))?;

        let event = crossterm::event::poll(Duration::from_millis(50))?;

        match event {
            true => match crossterm::event::read()? {
                Event::Key(key_event) => {
                    if model.is_edit_mode {
                        match key_event.code {
                            KeyCode::Esc => {
                                model.is_edit_mode = false;
                                textarea.clear();
                            }
                            KeyCode::Enter => {
                                let Some(current_board_id) = model.current_board_id else {
                                    // Show error!
                                    // We don't have errors so we just bail
                                    continue;
                                };
                                let todo_name = textarea.lines().join(" ").trim().to_string();

                                if let Some(index) = model.current_todo_index
                                    && !model.todos.is_empty()
                                {
                                    conn.reducers
                                        .update_todo(todo_name, model.todos[index].id)
                                        .ok();
                                } else {
                                    conn.reducers.add_todo(todo_name, current_board_id).ok();
                                }
                                model.is_edit_mode = false;
                                textarea.clear();
                            }
                            _ => {
                                textarea.input(key_event);
                            }
                        }
                    } else {
                        match key_event.code {
                            KeyCode::Char('q') => break Ok(()),
                            KeyCode::Left => update(&mut model, AppEvent::ChangeView(View::Boards)),
                            KeyCode::Right => update(&mut model, AppEvent::ChangeView(View::Todos)),
                            KeyCode::Up => match model.current_view {
                                View::Todos => move_up_todos(&mut model),
                                View::Boards => move_up_boards(conn, &model),
                            },
                            KeyCode::Down => match model.current_view {
                                View::Todos => move_down_todos(&mut model),
                                View::Boards => move_down_boards(conn, &model),
                            },
                            KeyCode::Enter => match model.current_view {
                                View::Todos => {
                                    model.is_edit_mode = true;
                                    if let Some(index) = model.current_todo_index
                                        && !model.todos.is_empty()
                                    {
                                        textarea.insert_str(&model.todos[index].name);
                                    }
                                }
                                View::Boards => {
                                    update(&mut model, AppEvent::ChangeView(View::Todos))
                                }
                            },
                            KeyCode::Char(' ') => match model.current_view {
                                View::Todos => {
                                    if let Some(index) = model.current_todo_index
                                        && !model.todos.is_empty()
                                    {
                                        conn.reducers.todo_done(model.todos[index].id).ok();
                                    }
                                }
                                View::Boards => {}
                            },
                            KeyCode::Char('a') => match model.current_view {
                                View::Todos => {
                                    model.is_edit_mode = true;
                                    model.current_todo_index = None;
                                }
                                View::Boards => {}
                            },
                            KeyCode::Char('d') => match model.current_view {
                                View::Todos => {
                                    if let Some(index) = model.current_todo_index
                                        && !model.todos.is_empty()
                                    {
                                        conn.reducers.delete_todo(model.todos[index].id).ok();
                                    }
                                }
                                View::Boards => {
                                    if let Some(board_id) = model.current_board_id {
                                        conn.reducers.delete_board(board_id).ok();
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn move_up_boards(conn: &DbConnection, model: &Model) {
    let selected_board_index = model
        .current_board_id
        .as_ref()
        .map(|current_board| model.boards.iter().position(|b| b.id == *current_board))
        .flatten()
        .unwrap_or(0);

    let new_index = if selected_board_index == 0 {
        model.boards.len() - 1
    } else {
        selected_board_index - 1
    };
    conn.reducers.view_board(model.boards[new_index].id).ok();
}

fn move_down_boards(conn: &DbConnection, model: &Model) {
    let selected_board_index = model
        .current_board_id
        .as_ref()
        .map(|current_board| model.boards.iter().position(|b| b.id == *current_board))
        .flatten()
        .unwrap_or(0);

    conn.reducers
        .view_board(model.boards[(selected_board_index + 1).rem_euclid(model.boards.len())].id)
        .ok();
}

fn move_up_todos(model: &mut Model) {
    let selected_todo_index = model.current_todo_index.unwrap_or(0);

    let new_index = if selected_todo_index == 0 {
        model.todos.len() - 1
    } else {
        selected_todo_index - 1
    };
    update(model, AppEvent::SelectTodoIndex(Some(new_index)));
}

fn move_down_todos(model: &mut Model) {
    let selected_todo_index = model.current_todo_index.unwrap_or(0);
    update(
        model,
        AppEvent::SelectTodoIndex(Some(
            (selected_todo_index + 1).rem_euclid(model.todos.len()),
        )),
    );
}

fn render(frame: &mut Frame, textarea: &TextArea, model: &mut Model) {
    let [left, right] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Length(50), Constraint::Fill(1)])
        .areas(frame.area());

    let left_block = Block::bordered();
    let left_area = left_block.inner(left);
    let right_block = Block::bordered();
    let right_area = right_block.inner(right);
    frame.render_widget(left_block, left);
    frame.render_widget(right_block, right);
    render_boards(frame, left_area, model);
    render_todos(frame, right_area, textarea, model);
}
