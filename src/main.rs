// SPDX-License-Identifier: MIT
//
// A simple webserver for COMP4000 experience 1.
// Copyright (c) 2021  William Findlay
//
// September 16, 2021  William Findlay  Created this.

use std::env;

use anyhow::{Context, Result};
use rand::{thread_rng, Rng};
use rocket::tokio;
use rocket::{catch, catchers, delete, get, post, put, routes, Config, State};
use rocket_contrib::json::Json;
use structopt::StructOpt;

use hello4000::*;

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

#[post("/fact")]
async fn create_fact(facts: &State<FactsContext>) -> String {
    todo!()
}

#[get("/fact")]
async fn read_fact(facts: &State<FactsContext>) -> String {
    todo!()
    // let i = thread_rng().gen_range(0..facts.len());
    // format!("New printer fact: {}\n", facts[i])
}

#[put("/fact")]
async fn update_fact(facts: &State<FactsContext>) -> String {
    todo!()
}

#[delete("/fact")]
async fn delete_fact(facts: &State<FactsContext>) -> String {
    todo!()
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Opt::from_args();

    // Cassandra setup
    let username = "student";
    let password = "student";
    let cassandra_ip = get_cassandra_addr().context("Failed to get Cassandra IP")?;
    let facts = FactsContext::new(username, password, &cassandra_ip)?;

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
        .register("/", catchers![error404])
        .manage(facts)
        .mount(
            "/",
            routes![
                index,
                ferris,
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
