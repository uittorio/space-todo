use spacetimedb::{Identity, Local, Query};
use spacetimedb::{ReducerContext, Table, ViewContext};

#[spacetimedb::table(accessor = user)]
pub struct User {
    #[primary_key]
    id: Identity,
    #[index(btree)]
    username: String,
    boards: Vec<u32>,

    #[index(btree)]
    current_board: u32,
}

#[spacetimedb::table(accessor = todo)]
pub struct Todo {
    #[primary_key]
    #[auto_inc]
    id: u32,
    name: String,
    done: bool,
    #[index(btree)]
    board_id: u32,
    created_by: Identity,
}

#[spacetimedb::table(accessor = board)]
pub struct Board {
    #[primary_key]
    #[auto_inc]
    id: u32,
    name: String,
    owner: Identity,
    participants: Vec<Identity>,
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    let token = ctx.sender_auth().jwt();
    if token
        .map(|token| token.issuer() == "localhost")
        .unwrap_or(true)
    {
        return Err("Anonymous cannot access this app".into());
    }

    const ADMINS_IDS: [&str; 2] = [
        "c2007e001f644897d350fd8a2de9197e78a48b92008d994dac86c38c8b0d399b",
        "c200f20bc8e521a559adc9d8c922d621a513f8f78f0dfd85f8e63c291c082445",
    ];

    const ADMIN_USERNAMES: [&str; 2] = ["uittorio", "pmyl"];

    let sender_id = ctx.sender().to_hex();
    if let Some(admin_index) = ADMINS_IDS.iter().position(|id| id == &sender_id.as_str()) {
        if let None = ctx.db.user().id().find(ctx.sender()) {
            ctx.db.user().insert(User {
                id: ctx.sender(),
                username: ADMIN_USERNAMES[admin_index].to_string(),
                boards: vec![],
                current_board: 0,
            });
        }

        Ok(())
    } else {
        return Err("This app can only be used by its admins".into());
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(_ctx: &ReducerContext) {
    // Called everytime a client disconnects
}

#[spacetimedb::reducer]
pub fn add_board(ctx: &ReducerContext, name: String) {
    let Some(mut user) = ctx.db.user().id().find(ctx.sender()) else {
        return;
    };

    let board = ctx.db.board().insert(Board {
        id: 0,
        name,
        owner: ctx.sender(),
        participants: vec![ctx.sender()],
    });

    user.boards.push(board.id);
    ctx.db.user().id().update(user);
}

#[spacetimedb::reducer]
pub fn update_board(ctx: &ReducerContext, name: String, id: u32) -> Result<(), String> {
    let Some(mut board) = ctx.db.board().id().find(id) else {
        return Ok(());
    };

    if !can_access_board(&ctx.db, ctx.sender(), board.id) {
        return Ok(());
    }

    board.name = name;
    ctx.db.board().id().update(board);
    Ok(())
}

#[spacetimedb::reducer]
pub fn delete_board(ctx: &ReducerContext, board_id: u32) -> Result<(), String> {
    let Some(board) = ctx.db.board().id().find(board_id) else {
        return Ok(());
    };

    if board.owner != ctx.sender() {
        return Ok(());
    }

    for mut user in board
        .participants
        .iter()
        .flat_map(|p| ctx.db.user().id().find(p))
    {
        if let Some(pos) = user.boards.iter().position(|&b_id| b_id == board.id) {
            user.boards.swap_remove(pos);
        }
        ctx.db.user().id().update(user);
    }

    for todo in ctx.db.todo().board_id().filter(board.id) {
        ctx.db.todo().id().delete(todo.id);
    }
    ctx.db.board().id().delete(board.id);

    Ok(())
}

#[spacetimedb::reducer]
pub fn view_board(ctx: &ReducerContext, board_id: u32) {
    if !can_access_board(&ctx.db, ctx.sender(), board_id) {
        return;
    }

    if let Some(mut user) = ctx.db.user().id().find(ctx.sender()) {
        user.current_board = board_id;
        ctx.db.user().id().update(user);
    }
}

#[spacetimedb::reducer]
pub fn assign_board(ctx: &ReducerContext, board_id: u32, user_id: Identity) {
    if !can_access_board(&ctx.db, ctx.sender(), board_id) {
        return;
    }

    let Some(mut user) = ctx.db.user().id().find(user_id) else {
        return;
    };

    let Some(mut board) = ctx.db.board().id().find(board_id) else {
        return;
    };

    if !user.boards.contains(&board_id) {
        user.boards.push(board_id);
    }

    if !board.participants.contains(&user.id) {
        board.participants.push(user.id);
    }

    ctx.db.user().id().update(user);
    ctx.db.board().id().update(board);
}

#[spacetimedb::reducer]
pub fn step_away_from_board(ctx: &ReducerContext) {
    if let Some(mut user) = ctx.db.user().id().find(ctx.sender()) {
        user.current_board = 0;
        ctx.db.user().id().update(user);
    }
}

#[spacetimedb::reducer]
pub fn add_todo(ctx: &ReducerContext, name: String, board_id: u32) -> Result<(), String> {
    if !can_access_board(&ctx.db, ctx.sender(), board_id) {
        return Err("Board not found".into());
    }

    ctx.db.todo().insert(Todo {
        id: 0,
        name,
        done: false,
        board_id,
        created_by: ctx.sender(),
    });

    Ok(())
}

#[spacetimedb::reducer]
pub fn update_todo(ctx: &ReducerContext, name: String, id: u32) -> Result<(), String> {
    let Some(mut todo) = ctx.db.todo().id().find(id) else {
        return Ok(());
    };

    if !can_access_board(&ctx.db, ctx.sender(), todo.board_id) {
        return Ok(());
    }

    todo.name = name;
    ctx.db.todo().id().update(todo);
    Ok(())
}

#[spacetimedb::reducer]
pub fn delete_todo(ctx: &ReducerContext, id: u32) -> Result<(), String> {
    let Some(todo) = ctx.db.todo().id().find(id) else {
        return Ok(());
    };

    if !can_access_board(&ctx.db, ctx.sender(), todo.board_id) {
        return Ok(());
    }

    ctx.db.todo().id().delete(id);

    Ok(())
}

#[spacetimedb::reducer]
pub fn todo_done(ctx: &ReducerContext, id: u32) -> Result<(), String> {
    let Some(mut todo) = ctx.db.todo().id().find(id) else {
        return Ok(());
    };

    if !can_access_board(&ctx.db, ctx.sender(), todo.board_id) {
        return Ok(());
    }

    todo.done = true;
    ctx.db.todo().id().update(todo);
    Ok(())
}

#[spacetimedb::reducer]
pub fn todo_undone(ctx: &ReducerContext, id: u32) -> Result<(), String> {
    let Some(mut todo) = ctx.db.todo().id().find(id) else {
        return Ok(());
    };

    if !can_access_board(&ctx.db, ctx.sender(), todo.board_id) {
        return Ok(());
    }

    todo.done = false;
    ctx.db.todo().id().update(todo);
    Ok(())
}

#[spacetimedb::view(accessor = my_boards, public)]
pub fn my_boards(ctx: &ViewContext) -> Vec<Board> {
    ctx.db
        .user()
        .id()
        .find(ctx.sender())
        .map(|user| {
            user.boards
                .iter()
                .filter_map(|board_id| ctx.db.board().id().find(board_id))
                .collect::<Vec<Board>>()
        })
        .unwrap_or_default()
}

#[spacetimedb::view(accessor = todos, public)]
pub fn todos(ctx: &ViewContext) -> impl Query<Todo> {
    let board_id = ctx
        .db
        .user()
        .id()
        .find(ctx.sender())
        .map(|user| user.current_board)
        .unwrap_or(0);
    // About the above 0, 0 is never an id
    // We can't do much here because views can't return Result or Option and the Query implementations have no default
    // This is the only way so that we return an empty list because no todo has board_id == 0

    ctx.from.todo().r#where(|todo| todo.board_id.eq(board_id))
}

#[spacetimedb::view(accessor = my_user, public)]
pub fn my_user(ctx: &ViewContext) -> impl Query<User> {
    ctx.from.user().r#where(|u| u.id.eq(ctx.sender()))
}

#[spacetimedb::view(accessor = current_board, public)]
pub fn current_board(ctx: &ViewContext) -> impl Query<Board> {
    ctx.from
        .user()
        .r#where(|u| u.id.eq(ctx.sender()))
        .right_semijoin(ctx.from.board(), |user, board| {
            user.current_board.eq(board.id)
        })
}

fn can_access_board(db: &Local, identity: Identity, board_id: u32) -> bool {
    db.user()
        .id()
        .find(identity)
        .map(|user| user.boards.contains(&board_id))
        .unwrap_or(false)
}
