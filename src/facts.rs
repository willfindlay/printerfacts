use anyhow::{Context, Result};
use cdrs::{
    authenticators::StaticPasswordAuthenticator,
    cluster::{session::new as new_session, ClusterTcpConfig, NodeTcpConfigBuilder},
    load_balancing::SingleNode,
};
use cdrs::{
    cluster::{session::Session, TcpConnectionPool},
    query::QueryExecutor,
    query_values,
};
use futures::StreamExt as _;
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio::sync::RwLock;
use std::sync::Arc;
use tokio_stream as stream;

#[derive(Serialize, Deserialize)]
pub struct Fact {
    uuid: String,
    fact: String,
    kind: String,
}

pub type CurrentSession = Session<SingleNode<TcpConnectionPool<StaticPasswordAuthenticator>>>;

pub struct FactsContext {
    session: Arc<RwLock<CurrentSession>>,
}

impl FactsContext {
    pub fn new(username: &str, password: &str, cassandra_ip: &str) -> Result<Self> {
        let auth = StaticPasswordAuthenticator::new(&username, &password);
        let nodes = vec![NodeTcpConfigBuilder::new(&cassandra_ip, auth).build()];
        let config = ClusterTcpConfig(nodes);
        let session =
            new_session(&config, SingleNode::new()).context("Failed to connect to Cassandra")?;
        Ok(Self {
            session: Arc::new(RwLock::new(session)),
        })
    }

    pub async fn create_fact(&self, fact: &str, kind: &str) -> Result<()> {
        const QUERY: &'static str = "INSERT INTO pfacts.facts \
            (key, fact, kind) VALUES (now(), ?, ?);";
        self.session
            .write()
            .await
            .query_with_values(QUERY, query_values!("fact" => fact, "kind" => kind))
            .context("Failed to create fact")?;
        Ok(())
    }

    pub async fn read_fact(&self, uuid: &str) -> Result<()> {
        todo!()
    }

    pub async fn update_fact(&self, uuid: &str) -> Result<()> {
        todo!()
    }

    pub async fn delete_fact(&self, uuid: &str) -> Result<()> {
        todo!()
    }

    pub async fn migrations(&self) -> Result<()> {
        self.create_keyspace().await?;
        self.create_table().await?;
        self.populate_facts().await?;
        Ok(())
    }

    /// Creates the keyspace for the Cassandra store.
    async fn create_keyspace(&self) -> Result<()> {
        const QUERY: &'static str = "CREATE KEYSPACE IF NOT EXISTS pfacts WITH REPLICATION = \
                { 'class' : 'SimpleStrategy', 'replication_factor' : 5};";
        self.session
            .write()
            .await
            .query(QUERY)
            .context("Key creation error")?;
        Ok(())
    }

    /// Creates the table to store facts.
    async fn create_table(&self) -> Result<()> {
        const QUERY: &'static str = "CREATE TABLE IF NOT EXISTS pfacts.facts \
                (key uuid PRIMARY KEY, fact varchar, kind varchar);";
        self.session
            .write()
            .await
            .query(QUERY)
            .context("Table creation error")?;
        Ok(())
    }

    /// Populate facts.
    async fn populate_facts(&self) -> Result<()> {
        let facts = pfacts::make();
        let mut facts_iter = stream::iter(facts.iter());
        while let Some(fact) = facts_iter.next().await {
            self.create_fact(&fact, "Cat fact").await?;
        }

        Ok(())
    }
}
