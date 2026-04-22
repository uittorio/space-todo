use std::{
    error::Error,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode};
use module_bindings::*;
use ratatui::{DefaultTerminal, style::Style};
use ratatui_textarea::TextArea;
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};

use crate::{
    dashboard::{
        render,
        state::{AppEvent, Model, View, update},
    },
    logs::Logger,
};

mod dashboard;
mod logs;
mod module_bindings;

enum RunError {
    Disconnected(Option<Box<dyn Error>>),
    Other(Box<dyn Error>),
}

pub fn run(id_token: String, host: String, db_name: String) -> Result<(), Box<dyn Error>> {
    let mut logger = Logger::default();
    logger.info("App started");

    let mut burst_disconnections_count = 0;
    // Max length of time for the disconnection to be considered "burst"
    const BURST_MS: Duration = Duration::from_millis(1000);
    const MAX_BURST_DISCONNECTIONS: usize = 10;

    loop {
        let start_time = Instant::now();

        match run_with_connection(id_token.clone(), host.clone(), db_name.clone(), &mut logger) {
            Ok(_) => break,
            Err(RunError::Disconnected(e)) => {
                logger.error(e.map(|e| e.to_string()).unwrap_or("Unknown error".into()));

                if Instant::now() - start_time < BURST_MS {
                    burst_disconnections_count += 1;
                } else {
                    burst_disconnections_count = 0;
                }

                if burst_disconnections_count > MAX_BURST_DISCONNECTIONS {
                    return Err(format!(
                        "Disconnected more than {} times, bail",
                        MAX_BURST_DISCONNECTIONS
                    )
                    .into());
                }
            }
            Err(RunError::Other(e)) => return Err(e),
        }
    }

    Ok(())
}

fn run_with_connection(
    id_token: String,
    host: String,
    db_name: String,
    logger: &mut Logger,
) -> Result<(), RunError> {
    let (tx, rx): (Sender<AppEvent>, Receiver<AppEvent>) = mpsc::channel();
    let (disconnect_tx, disconnect_rx): (
        Sender<Option<spacetimedb_sdk::Error>>,
        Receiver<Option<spacetimedb_sdk::Error>>,
    ) = mpsc::channel();

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
        .on_disconnect(move |_ctx, e| {
            eprintln!("Disconnected {:?}", e);
            disconnect_tx.send(e).ok();
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

    conn.db.my_boards().on_delete({
        let tx = tx.clone();
        move |_, r| {
            tx.send(AppEvent::OnBoardDeleted(r.clone())).ok();
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

    color_eyre::install().map_err(|e| RunError::Other(e.into()))?;

    ratatui::run(|terminal| app(terminal, rx, disconnect_rx, &conn, logger))?;

    conn.disconnect().map_err(|e| RunError::Other(e.into()))?;

    Ok(())
}

fn app(
    terminal: &mut DefaultTerminal,
    receiver: Receiver<AppEvent>,
    disconnect_receiver: Receiver<Option<spacetimedb_sdk::Error>>,
    conn: &DbConnection,
    logger: &mut Logger,
) -> Result<(), RunError> {
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
        logger: logger,
    };

    loop {
        if let Ok(e) = disconnect_receiver.try_recv() {
            return Err(RunError::Disconnected(e.map(|e| e.into())));
        }

        for event in receiver.try_iter() {
            update(&mut model, event);
        }

        terminal
            .draw(|frame| render(frame, &mut textarea, &mut model))
            .map_err(|e| RunError::Other(e.into()))?;

        let event = crossterm::event::poll(Duration::from_millis(50))
            .map_err(|e| RunError::Other(e.into()))?;

        match event {
            true => match crossterm::event::read().map_err(|e| RunError::Other(e.into()))? {
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
                            KeyCode::Esc => {
                                if let View::Logs = model.current_view {
                                    update(&mut model, AppEvent::ChangeView(View::Boards));
                                }
                            }
                            KeyCode::Char('q') => break Ok(()),
                            KeyCode::Char('l') => {
                                if let View::Logs = model.current_view {
                                    update(&mut model, AppEvent::ChangeView(View::Boards));
                                } else {
                                    update(&mut model, AppEvent::ChangeView(View::Logs))
                                }
                            }
                            KeyCode::Left => update(&mut model, AppEvent::ChangeView(View::Boards)),
                            KeyCode::Right => update(&mut model, AppEvent::ChangeView(View::Todos)),
                            KeyCode::Enter => match model.current_view {
                                View::Todos => {}
                                View::Boards => {
                                    update(&mut model, AppEvent::ChangeView(View::Todos))
                                }
                                View::Logs => {}
                            },
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
                                View::Logs => {}
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
