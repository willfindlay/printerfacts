// SPDX-License-Identifier: MIT
//
// A simple webserver for COMP4000 experience 1.
// Copyright (c) 2021  William Findlay
//
// September 16, 2021  William Findlay  Created this.

use std::env;

use anyhow::{Context, Result};
use rand::{seq::SliceRandom, thread_rng};
use rocket::tokio;
use rocket::{catch, catchers, delete, get, post, put, routes, Config, State};
use rocket::{serde::json::Json, serde::uuid::Uuid};
use structopt::StructOpt;

use hello4000::*;
//use uuid::Uuid;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    migrations: bool,
}

#[get("/")]
async fn index() -> String {
    format!(
        "Hello k8s world! I am a simple server running on node {} in pod {}\n",
        get_nodename(),
        get_hostname()
    )
}

#[get("/fact/keys")]
async fn get_keys(facts: &State<FactsContext>) -> Result<Json<Vec<Uuid>>, String> {
    Ok(Json(
        facts.get_keys().await.map_err(|e| format!("{:?}", e))?,
    ))
}

#[post("/fact", format = "json", data = "<fact>")]
async fn create_fact(fact: Json<Fact>, facts: &State<FactsContext>) -> Result<(), String> {
    facts
        .create_fact(&fact.0.fact, &fact.0.kind)
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

#[get("/fact/<key>")]
async fn read_fact(key: Uuid, facts: &State<FactsContext>) -> Result<Json<Fact>, String> {
    Ok(Json(
        facts.read_fact(key).await.map_err(|e| format!("{:?}", e))?,
    ))
}

#[get("/fact")]
async fn random_fact(facts: &State<FactsContext>) -> Result<Json<Fact>, String> {
    let keys = facts.get_keys().await.map_err(|e| format!("{:?}", e))?;
    let key = keys
        .choose(&mut thread_rng())
        .ok_or("Failed to choose random key")?;
    Ok(Json(
        facts
            .read_fact(key.clone())
            .await
            .map_err(|e| format!("{:?}", e))?,
    ))
}

#[put("/fact/<key>", format = "json", data = "<fact>")]
async fn update_fact(
    key: Uuid,
    fact: Json<Fact>,
    facts: &State<FactsContext>,
) -> Result<(), String> {
    facts
        .update_fact(&fact.0.fact, &fact.0.kind, key)
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

#[delete("/fact/<key>")]
async fn delete_fact(key: Uuid, facts: &State<FactsContext>) -> Result<(), String> {
    facts
        .delete_fact(key)
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

#[get("/crashme")]
fn crashme() -> String {
    eprintln!("It's not a bug, it's a feature!\n");
    std::process::exit(1);
}

#[get("/ferris")]
async fn ferris() -> &'static str {
    "🦀 This app was written in Rust 🦀\n".into()
}

#[get("/credit")]
async fn credit() -> &'static str {
    "Printer facts are from the `pfacts` crate by Christine Dodrill.\n"
}

#[catch(404)]
async fn error404() -> &'static str {
    r#"This is not the URI you are looking for.
                       .-.
                      |_:_|
                     /(_Y_)\
.                   ( \/M\/ )
 '.               _.'-/'-'\-'._
   ':           _/.--'[[[[]'--.\_
     ':        /_'  : |::"| :  '.\
       ':     //   ./ |oUU| \.'  :\
         ':  _:'..' \_|___|_/ :   :|
           ':.  .'  |_[___]_|  :.':\
            [::\ |  :  | |  :   ; : \
             '-'   \/'.| |.' \  .;.' |
             |\_    \  '-'   :       |
             |  \    \ .:    :   |   |
             |   \    | '.   :    \  |
             /       \   :. .;       |
            /     |   |  :__/     :  \\
           |  |   |    \:   | \   |   ||
          /    \  : :  |:   /  |__|   /|
      snd |     : : :_/_|  /'._\  '--|_\
          /___.-/_|-'   \  \
                         '-'
                            Art by Shanaka Dias
    "#
}

#[catch(500)]
async fn error500() -> &'static str {
    r#"It's not a bug, it's a feature! (Internal Server Error)
    "#
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Opt::from_args();

    // Cassandra setup
    // let username = "student";
    // let password = "student";
    let cassandra_ip = get_cassandra_addr().context("Failed to get Cassandra IP")?;
    let facts = FactsContext::new(&cassandra_ip)?;

    if args.migrations {
        facts
            .migrations()
            .await
            .context("Failed to run migrations")?;
        return Ok(());
    }

    launch(facts).await
}

async fn launch(facts: FactsContext) -> Result<()> {
    let figment = Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 4000u32));

    rocket::custom(figment)
        .attach(Counter::default())
        .register("/", catchers![error404, error500])
        .manage(facts)
        .mount(
            "/",
            routes![
                index,
                ferris,
                get_keys,
                random_fact,
                create_fact,
                read_fact,
                update_fact,
                delete_fact,
                credit,
                crashme
            ],
        )
        .launch()
        .await?;

    Ok(())
}

fn get_cassandra_addr() -> Result<String> {
    let ip = env::var("CASSANDRA_IP").context("No CASSANDRA_IP in environment")?;
    Ok(format!("{}:9042", ip))
}
