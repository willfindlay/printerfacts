// SPDX-License-Identifier: MIT
//
// A simple webserver for COMP4000 experience 1.
// Copyright (c) 2021  William Findlay
//
// September 16, 2021  William Findlay  Created this.

use std::io::Read;
use std::{env, process::Command};

use anyhow::Result;
use cdrs::{
    authenticators::StaticPasswordAuthenticator,
    cluster::{session::new as new_session, ClusterTcpConfig, NodeTcpConfig, NodeTcpConfigBuilder},
    load_balancing::SingleNode,
};
use rand::{thread_rng, Rng};
use rocket::{catch, catchers, delete, get, launch, post, put, routes, Config, State};
use rocket_contrib::json::Json;

use hello4000::*;

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

#[launch]
async fn rocket() -> _ {
    let figment = Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 4000u32));

    // Cassandra setup
    let username = "cassandra";
    let password = "1337h4x0r";
    let cassandra_ip = get_cassandra_ip().expect("No Cassandra IP in environment");
    let auth = StaticPasswordAuthenticator::new(&username, &password);
    let nodes = vec![NodeTcpConfigBuilder::new(&cassandra_ip, auth).build()];
    let config = ClusterTcpConfig(nodes);
    let session = new_session(&config, SingleNode::new()).expect("Failed to connect to Cassandra");
    let facts = FactsContext::new(session).expect("Failed to spawn connection to Cassandra");

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
}

fn get_cassandra_ip() -> Result<String> {
    let kube_args = vec![
        "get",
        "svc",
        "--namespace",
        "default",
        "cassandra",
        "--template",
        "{{ range (index .status.loadBalancer.ingress 0) }}{{.}}{{ end }}",
    ];
    let ip = String::from_utf8(Command::new("kubectl").args(kube_args).output()?.stdout)?;
    Ok(format!("{}:9042", ip))
}
