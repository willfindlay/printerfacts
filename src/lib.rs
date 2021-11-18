mod facts;
mod fairings;

pub use facts::FactsContext;
pub use fairings::Counter;
use std::env;

/// Get the hostname and return it as a string
pub fn get_hostname() -> String {
    gethostname::gethostname().to_string_lossy().into()
}

/// Get the node from an environment variable and return it as a string
pub fn get_nodename() -> String {
    env::var("NODE_NAME").unwrap_or("UNKNOWN".into())
}
