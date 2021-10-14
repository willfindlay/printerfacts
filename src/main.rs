// SPDX-License-Identifier: MIT
//
// A simple webserver for COMP4000 experience 1.
// Copyright (c) 2021  William Findlay
//
// September 16, 2021  William Findlay  Created this.

use rand::{thread_rng, Rng};
use rocket::{catch, catchers, get, launch, routes, Config, State};

use hello4000::*;

#[get("/")]
async fn index() -> String {
    format!(
        "Hello k8s world! I am a simple server running on pod {}\n",
        get_hostname().await
    )
}

#[get("/printerfacts")]
async fn fact(facts: &State<pfacts::Facts>) -> String {
    let i = thread_rng().gen_range(0..facts.len());
    format!("New printer fact: {}\n", facts[i])
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
        .merge(("port", 4000));
    let facts = pfacts::make();

    rocket::custom(figment)
        .attach(Counter::default())
        .register("/", catchers![error404])
        .manage(facts)
        .mount("/", routes![index, ferris, fact, credit, crashme])
}
