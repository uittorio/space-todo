use crate::module_bindings::{Board, Todo};

pub struct Model {
    pub boards: Vec<Board>,
    pub todos: Vec<Todo>,
    pub current_board_id: Option<u32>,
    pub current_view: View,
    pub current_todo_index: Option<usize>,
    pub is_edit_mode: bool,
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
}

pub fn update(model: &mut Model, event: AppEvent) {
    match event {
        AppEvent::OnBoardAdded(board) => {
            model.boards.push(board);
        }
        AppEvent::OnTodoAdded(todo) => {
            model.todos.push(todo);
        }
        AppEvent::OnTodoUpdated(todo) => {
            if let Some(index) = model.todos.iter().position(|t| t.id == todo.id) {
                model.todos[index] = todo;
            }
        }
        AppEvent::OnBoardDeleted(board) => {
            if let Some(index) = model.boards.iter().position(|b| b.id == board.id) {
                model.boards.remove(index);
            }
        }
        AppEvent::OnTodoDeleted(todo) => {
            if let Some(index) = model.todos.iter().position(|b| b.id == todo.id) {
                model.todos.remove(index);
            }
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
    }
}
