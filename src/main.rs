mod module_bindings;
use crossterm::event::{Event, KeyCode};
use dotenv::dotenv;
use module_bindings::*;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Paragraph},
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
    let mut textarea = TextArea::new(vec![]);
    textarea.set_placeholder_text("Type to create");
    textarea.set_style(Style::new().on_black());

    let mut model = Model {
        boards: vec![],
        todos: vec![],
        current_board_id: None,
        current_view: View::Boards,
        current_todo_index: None,
        is_edit_mode: false,
        conn,
    };

    loop {
        for event in receiver.try_iter() {
            update(&mut model, event);
        }
        terminal.draw(|frame| render(frame, &mut textarea, &mut model))?;

        let event = crossterm::event::poll(Duration::from_millis(50))?;

        match event {
            true => match crossterm::event::read()? {
                Event::Key(key_event) => {
                    if model.is_edit_mode {
                        match key_event.code {
                            KeyCode::Esc => {
                                update(&mut model, AppEvent::CloseEditMode);
                                textarea.clear();
                            }
                            KeyCode::Enter => {
                                let todo_name = textarea.lines().join(" ").trim().to_string();
                                update(&mut model, AppEvent::AddOrUpdateItem(todo_name));
                                update(&mut model, AppEvent::CloseEditMode);
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
                            KeyCode::Up => update(&mut model, AppEvent::MoveUpInView),
                            KeyCode::Down => update(&mut model, AppEvent::MoveDownInView),
                            KeyCode::Char('e') => match model.current_view {
                                View::Todos => {
                                    update(&mut model, AppEvent::EditMode);
                                    if let Some(index) = model.current_todo_index
                                        && !model.todos.is_empty()
                                    {
                                        textarea.insert_str(&model.todos[index].name);
                                    };
                                }
                                View::Boards => {
                                    update(&mut model, AppEvent::EditMode);

                                    if let Some(id) = model.current_board_id {
                                        if let Some(index) =
                                            model.boards.iter().position(|b| b.id == id)
                                        {
                                            textarea.insert_str(&model.boards[index].name);
                                        }
                                    }
                                }
                            },
                            KeyCode::Char(' ') => update(&mut model, AppEvent::Toggle),
                            KeyCode::Char('a') => update(&mut model, AppEvent::Add),
                            KeyCode::Char('d') => update(&mut model, AppEvent::Delete),
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

fn render(frame: &mut Frame, textarea: &mut TextArea, model: &mut Model) {
    let [top, bottom] = Layout::default()
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

    let [left, right] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(50), Constraint::Fill(1)])
        .areas(bottom);

    let left_block = Block::bordered();
    let left_area = left_block.inner(left);

    let right_block = Block::bordered();
    let right_area = right_block.inner(right);

    frame.render_widget(left_block, left);
    frame.render_widget(right_block, right);

    render_boards(frame, left_area, textarea, model);
    render_todos(frame, right_area, textarea, model);
}
