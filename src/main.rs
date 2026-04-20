use dotenv::dotenv;
use space_todo::run;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let id_token = env::var("SPACETIMEDB_TOKEN").unwrap();

    // The URI of the SpacetimeDB instance hosting our chat module.
    let host: String =
        env::var("SPACETIMEDB_HOST").unwrap_or("https://maincloud.spacetimedb.com".to_string());

    // The module name we chose when we published our module.
    let db_name: String = env::var("SPACETIMEDB_DB_NAME").unwrap_or("space-todo-fn915".to_string());

    run(id_token, host, db_name)?;

    Ok(())
}
