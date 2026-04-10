use log::info;
use spacetimedb::{Errno, Query};
use spacetimedb::{ReducerContext, Table, ViewContext};

#[spacetimedb::table(accessor = person)]
pub struct Person {
    name: String,
}

#[spacetimedb::table(accessor = todo)]
pub struct Todo {
    #[primary_key]
    #[auto_inc]
    id: u32,
    name: String,
    done: bool,
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(_ctx: &ReducerContext) -> Result<(), String> {
    let t = _ctx.sender_auth().jwt();
    log::info!("{:?}", t.map(|f| f.()));
    if _ctx.sender_auth().jwt().is_none() {
        return Err("No ano".into());
    }

    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(_ctx: &ReducerContext) {
    // Called everytime a client disconnects
}

#[spacetimedb::reducer]
pub fn add(ctx: &ReducerContext, name: String) {
    ctx.db.person().insert(Person { name });
}

#[spacetimedb::reducer]
pub fn say_hello(ctx: &ReducerContext) {
    for person in ctx.db.person().iter() {
        log::info!("Hello, {}!", person.name);
    }
    log::info!("Hello, World!");
}

#[spacetimedb::reducer]
pub fn add_todo(ctx: &ReducerContext, name: String) {
    ctx.db.todo().insert(Todo {
        id: 0,
        name,
        done: false,
    });
}

#[spacetimedb::reducer]
pub fn todo_done(ctx: &ReducerContext, id: u32) {
    if let Some(mut todo) = ctx.db.todo().id().find(id) {
        todo.done = true;
        ctx.db.todo().id().update(todo);
    }
}

#[spacetimedb::view(accessor = todos, public)]
pub fn todos(ctx: &ViewContext) -> impl Query<Todo> {
    ctx.from.todo()
}
