use anyhow::{Context, Result};
use cdrs::{
    authenticators::StaticPasswordAuthenticator,
    cluster::{session::Session, TcpConnectionPool},
    load_balancing::SingleNode,
    query::QueryExecutor,
};
use rocket::tokio::sync::RwLock;
use std::sync::Arc;

pub type CurrentSession = Session<SingleNode<TcpConnectionPool<StaticPasswordAuthenticator>>>;

pub struct FactsContext {
    session: Arc<RwLock<CurrentSession>>,
}

impl FactsContext {
    pub fn new(mut session: CurrentSession) -> Result<Self> {
        Self::create_keyspace(&mut session)?;

        Ok(Self {
            session: Arc::new(RwLock::new(session)),
        })
    }

    /// Creates the keyspace for the Cassandra store.
    fn create_keyspace(session: &mut CurrentSession) -> Result<()> {
        const CREATE_KEYS: &'static str =
            "CREATE KEYSPACE IF NOT EXISTS pfacts_ks WITH REPLICATION = { \
                            'class' : 'SimpleStrategy', 'replication_factor' : 5};";
        session.query(CREATE_KEYS).context("Key creation error")?;
        Ok(())
    }
}
