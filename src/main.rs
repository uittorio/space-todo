mod module_bindings;
use dotenv::dotenv;
use module_bindings::*;
use std::env;

use spacetimedb_sdk::{DbContext, Table};

fn main() {
    dotenv().ok();
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

    conn.run_threaded();

    // Subscribe to the person table
    conn.subscription_builder()
        .on_applied(|_ctx| println!("Subscripted to the person table"))
        .on_error(|_ctx, e| eprintln!("There was an error when subscring to the person table: {e}"))
        .add_query(|q| q.from.person())
        .subscribe();

    // Register a callback for when rows are inserted into the person table
    conn.db().person().on_insert(|_ctx, person| {
        println!("New person: {}", person.name);
    });

    // Keep the main thread alive so the connection stays open
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
