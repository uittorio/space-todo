use crate::module_bindings::{self, Board, DbConnection, Todo};
use module_bindings::*;

pub struct Model<'a> {
    pub boards: Vec<Board>,
    pub todos: Vec<Todo>,
    pub current_board_id: Option<u32>,
    pub current_view: View,
    pub current_todo_index: Option<usize>,
    pub is_edit_mode: bool,
    pub conn: &'a DbConnection,
    pub last_error: Option<String>,
}

pub enum View {
    Todos,
    Boards,
}

pub enum AppEvent {
    OnBoardAdded(Board),
    OnBoardDeleted(Board),
    OnTodoAdded(Todo),
    OnTodoDeleted(Todo),
    OnTodoUpdated(Todo),
    OnCurrentBoardUpdated(Option<u32>),
    ChangeView(View),
    SelectTodoIndex(Option<usize>),
    EditMode,
    CloseEditMode,
    AddOrUpdateItem(String),
    MoveUpInView,
    MoveDownInView,
    Toggle,
    Add,
    Delete,
}

pub fn update(model: &mut Model, event: AppEvent) {
    if let Err(error) = update_internal(model, event) {
        model.last_error = Some(error);
    }
}

fn update_internal(model: &mut Model, event: AppEvent) -> Result<(), String> {
    match event {
        AppEvent::OnBoardAdded(board) => {
            model.boards.push(board);
        }
        AppEvent::OnBoardDeleted(board) => {
            if let Some(index) = model.boards.iter().position(|b| b.id == board.id) {
                model.boards.remove(index);
            }
        }
        AppEvent::OnTodoAdded(todo) => {
            model.todos.push(todo);
            model.todos.sort_by_key(|t| t.done);
        }
        AppEvent::OnTodoUpdated(todo) => {
            if let Some(index) = model.todos.iter().position(|t| t.id == todo.id) {
                model.todos[index] = todo;
            }
            model.todos.sort_by_key(|t| t.done);
        }
        AppEvent::OnTodoDeleted(todo) => {
            if let Some(index) = model.todos.iter().position(|b| b.id == todo.id) {
                model.todos.remove(index);
            }
            model.todos.sort_by_key(|t| t.done);
        }
        AppEvent::OnCurrentBoardUpdated(board) => {
            model.current_board_id = board;
        }
        AppEvent::ChangeView(view) => {
            model.current_view = view;

            match model.current_view {
                View::Todos => model.current_todo_index = Some(0),
                View::Boards => model.current_todo_index = None,
            }
        }
        AppEvent::SelectTodoIndex(todo_index) => model.current_todo_index = todo_index,
        AppEvent::CloseEditMode => {
            model.is_edit_mode = false;
        }
        AppEvent::EditMode => {
            model.is_edit_mode = true;
        }
        AppEvent::AddOrUpdateItem(name) => match model.current_view {
            View::Todos => {
                let Some(current_board_id) = model.current_board_id else {
                    // Show error!
                    // We don't have errors so we just bail
                    return Ok(());
                };

                if let Some(index) = model.current_todo_index
                    && !model.todos.is_empty()
                {
                    model
                        .conn
                        .reducers
                        .update_todo(name, model.todos[index].id)
                        .map_err(|e| e.to_string())?;
                } else {
                    model
                        .conn
                        .reducers
                        .add_todo(name, current_board_id)
                        .map_err(|e| e.to_string())?;
                }
            }
            View::Boards => {
                if let Some(id) = model.current_board_id
                    && !model.boards.is_empty()
                {
                    model
                        .conn
                        .reducers
                        .update_board(name, id)
                        .map_err(|e| e.to_string())?;
                } else {
                    model
                        .conn
                        .reducers
                        .add_board(name)
                        .map_err(|e| e.to_string())?;
                }
            }
        },
        AppEvent::MoveUpInView => match model.current_view {
            View::Todos => move_up_todos(model),
            View::Boards => move_up_boards(model.conn, model)?,
        },
        AppEvent::MoveDownInView => match model.current_view {
            View::Todos => move_down_todos(model),
            View::Boards => move_down_boards(model.conn, model)?,
        },
        AppEvent::Toggle => match model.current_view {
            View::Todos => {
                if let Some(index) = model.current_todo_index
                    && !model.todos.is_empty()
                {
                    if model.todos[index].done {
                        model
                            .conn
                            .reducers
                            .todo_undone(model.todos[index].id)
                            .map_err(|e| e.to_string())?;
                    } else {
                        model
                            .conn
                            .reducers
                            .todo_done(model.todos[index].id)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            View::Boards => {}
        },
        AppEvent::Add => match model.current_view {
            View::Todos => {
                model.is_edit_mode = true;
                model.current_todo_index = None;
            }
            View::Boards => {
                model.is_edit_mode = true;
                model.current_board_id = None;
            }
        },
        AppEvent::Delete => match model.current_view {
            View::Todos => {
                if let Some(index) = model.current_todo_index
                    && !model.todos.is_empty()
                {
                    model
                        .conn
                        .reducers
                        .delete_todo(model.todos[index].id)
                        .map_err(|e| e.to_string())?;
                }
            }
            View::Boards => {
                if let Some(board_id) = model.current_board_id {
                    model
                        .conn
                        .reducers
                        .delete_board(board_id)
                        .map_err(|e| e.to_string())?;
                }
            }
        },
    }

    Ok(())
}

fn move_up_boards(conn: &DbConnection, model: &Model) -> Result<(), String> {
    if model.boards.is_empty() {
        return Ok(());
    }

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
    conn.reducers
        .view_board(model.boards[new_index].id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn move_down_boards(conn: &DbConnection, model: &Model) -> Result<(), String> {
    if model.boards.is_empty() {
        return Ok(());
    }

    let selected_board_index = model
        .current_board_id
        .as_ref()
        .map(|current_board| model.boards.iter().position(|b| b.id == *current_board))
        .flatten()
        .unwrap_or(0);

    conn.reducers
        .view_board(model.boards[(selected_board_index + 1).rem_euclid(model.boards.len())].id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn move_up_todos(model: &mut Model) {
    if model.todos.is_empty() {
        return;
    }

    let selected_todo_index = model.current_todo_index.unwrap_or(0);

    let new_index = if selected_todo_index == 0 {
        model.todos.len() - 1
    } else {
        selected_todo_index - 1
    };
    update(model, AppEvent::SelectTodoIndex(Some(new_index)));
}

fn move_down_todos(model: &mut Model) {
    if model.todos.is_empty() {
        return;
    }

    let selected_todo_index = model.current_todo_index.unwrap_or(0);
    update(
        model,
        AppEvent::SelectTodoIndex(Some(
            (selected_todo_index + 1).rem_euclid(model.todos.len()),
        )),
    );
}
