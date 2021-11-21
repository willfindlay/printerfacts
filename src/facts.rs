use anyhow::{anyhow, Context, Result};
use cdrs::{
    authenticators::StaticPasswordAuthenticator,
    cluster::{session::new as new_session, ClusterTcpConfig, NodeTcpConfigBuilder},
    consistency::Consistency,
    load_balancing::SingleNode,
    query::QueryParamsBuilder,
    types::from_cdrs::FromCDRSByName,
    types::prelude::*,
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
use uuid::Uuid;

#[derive(Serialize, Deserialize, TryFromRow)]
pub struct Fact {
    #[serde(skip_deserializing)]
    key: Uuid,
    pub fact: String,
    pub kind: String,
}

#[derive(TryFromRow)]
struct FactKeysRow {
    key: Uuid,
}

pub type CurrentSession = Session<SingleNode<TcpConnectionPool<StaticPasswordAuthenticator>>>;

pub struct FactsContext {
    session: Arc<RwLock<CurrentSession>>,
    consistency: Consistency,
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
            consistency: Consistency::All,
        })
    }

    pub async fn get_keys(&self) -> Result<Vec<Uuid>> {
        const QUERY: &'static str = "SELECT key FROM pfacts.facts;";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .finalize();
        let rows = self
            .session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?
            .get_body()
            .context("Get body failed")?
            .into_rows()
            .ok_or(anyhow!("Into rows failed"))?;

        let mut keys = vec![];
        for row in rows {
            keys.push(
                FactKeysRow::try_from_row(row)
                    .context("Try from row failed")?
                    .key,
            );
        }
        Ok(keys)
    }

    pub async fn create_fact(&self, fact: &str, kind: &str) -> Result<()> {
        const QUERY: &'static str = "INSERT INTO pfacts.facts \
            (key, fact, kind) VALUES (now(), ?, ?);";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .values(query_values!("fact" => fact, "kind" => kind))
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?;
        Ok(())
    }

    pub async fn read_fact(&self, key: Uuid) -> Result<Fact> {
        const QUERY: &'static str = "SELECT * from pfacts.facts \
            WHERE key = ?;";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .values(query_values!("key" => key))
            .finalize();
        let rows = self
            .session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?
            .get_body()
            .context("Get body failed")?
            .into_rows()
            .ok_or(anyhow!("Into rows failed"))?;
        Ok(Fact::try_from_row(
            rows.get(0).context("No such fact")?.to_owned(),
        )?)
    }

    pub async fn update_fact(&self, fact: &str, kind: &str, key: Uuid) -> Result<()> {
        const QUERY: &'static str = "UPDATE pfacts.facts \
            SET (fact, kind) = (?, ?) WHERE key = ?;";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .values(query_values!("fact" => fact, "kind" => kind, "key" => key))
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?;
        Ok(())
    }

    pub async fn delete_fact(&self, key: Uuid) -> Result<()> {
        const QUERY: &'static str = "DELETE FROM pfacts.facts WHERE key = ?;";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .values(query_values!("key" => key))
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?;
        Ok(())
    }

    pub async fn migrations(&self) -> Result<()> {
        self.create_keyspace().await?;
        self.create_table().await?;
        self.populate_facts().await?;
        Ok(())
    }

    /// Creates the keyspace for the Cassandra store.
    async fn create_keyspace(&self) -> Result<()> {
        const QUERY: &'static str = "CREATE KEYSPACE IF NOT EXISTS pfacts \
                WITH REPLICATION = {'class' : 'SimpleStrategy', 'replication_factor' : 2};";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?;
        Ok(())
    }

    /// Creates the table to store facts.
    async fn create_table(&self) -> Result<()> {
        const DROP: &'static str = "DROP TABLE IF EXISTS pfacts.facts;";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(DROP, params)
            .context("Query failed")?;

        const CREATE: &'static str = "CREATE TABLE IF NOT EXISTS pfacts.facts \
                (key uuid PRIMARY KEY, fact varchar, kind varchar);";
        let params = QueryParamsBuilder::new()
            .consistency(self.consistency)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(CREATE, params)
            .context("Query failed")?;

        Ok(())
    }

    /// Populate facts.
    async fn populate_facts(&self) -> Result<()> {
        let facts = pfacts::make();
        let mut facts_iter = stream::iter(facts.iter());
        while let Some(fact) = facts_iter.next().await {
            self.create_fact(&fact, "Cat fact").await?;
        }

        // const QUERY: &'static str = "INSERT INTO pfacts.facts \
        //     (key, fact, kind) VALUES (now(), ?, ?);";

        // let mut batch = BatchQueryBuilder::new().consistency(self.consistency.read().await.clone());

        // let facts = pfacts::make();
        // let mut facts_iter = stream::iter(facts.iter());
        // while let Some(fact) = facts_iter.next().await {
        //     println!("new fact");
        //     batch = batch.add_query(
        //         QUERY,
        //         query_values!("fact" => fact.as_str(), "kind" => "Cat fact"),
        //     );
        // }

        // let batch = batch.finalize().context("Failed to prepare batch query")?;
        // self.session
        //     .write()
        //     .await
        //     .batch_with_params(batch)
        //     .context("Query failed")?;

        Ok(())
    }
}
