mod fairings;

pub use fairings::Counter;

/// Get the hostname and return it as a string
pub async fn get_hostname() -> String {
    gethostname::gethostname().to_string_lossy().into()
}
