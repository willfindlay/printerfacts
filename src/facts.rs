use anyhow::{anyhow, Context, Result};
use cdrs::{
    authenticators::NoneAuthenticator,
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

pub type CurrentSession = Session<SingleNode<TcpConnectionPool<NoneAuthenticator>>>;

pub struct FactsContext {
    session: Arc<RwLock<CurrentSession>>,
    read_consistency: Arc<RwLock<Consistency>>,
    write_consistency: Arc<RwLock<Consistency>>,
    replica_count: u32,
}

impl FactsContext {
    pub fn new(cassandra_ip: &str) -> Result<Self> {
        let auth = NoneAuthenticator;
        let nodes = vec![NodeTcpConfigBuilder::new(&cassandra_ip, auth).build()];
        let config = ClusterTcpConfig(nodes);
        let session =
            new_session(&config, SingleNode::new()).context("Failed to connect to Cassandra")?;
        Ok(Self {
            session: Arc::new(RwLock::new(session)),
            read_consistency: Arc::new(RwLock::new(Consistency::One)),
            write_consistency: Arc::new(RwLock::new(Consistency::LocalQuorum)),
            replica_count: 2,
        })
    }

    async fn read_consistency(&self) -> Consistency {
        self.read_consistency.read().await.clone()
    }

    async fn write_consistency(&self) -> Consistency {
        self.write_consistency.write().await.clone()
    }

    pub async fn get_keys(&self) -> Result<Vec<Uuid>> {
        const QUERY: &'static str = "SELECT key FROM pfacts.facts;";
        let params = QueryParamsBuilder::new().finalize();
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
            .values(query_values!("fact" => fact, "kind" => kind))
            .consistency(self.write_consistency().await)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context(format!("Query failed with fact={:?} kind={:?}", fact, kind))?;
        Ok(())
    }

    pub async fn read_fact(&self, key: Uuid) -> Result<Fact> {
        const QUERY: &'static str = "SELECT * from pfacts.facts \
            WHERE key = ?;";
        let params = QueryParamsBuilder::new()
            .values(query_values!("key" => key))
            .consistency(self.read_consistency().await)
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
            SET fact = ?, kind = ? WHERE key = ?;";
        let params = QueryParamsBuilder::new()
            .values(query_values!("fact" => fact, "kind" => kind, "key" => key))
            .consistency(self.write_consistency().await)
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
            .values(query_values!("key" => key))
            .consistency(self.write_consistency().await)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(QUERY, params)
            .context("Query failed")?;
        Ok(())
    }

    pub async fn migrations(&self) -> Result<()> {
        self.create_keyspace()
            .await
            .context("Failed to create keyspace")?;
        self.create_table()
            .await
            .context("Failed to create table")?;
        self.populate_facts()
            .await
            .context("Failed to populate facts")?;
        Ok(())
    }

    /// Creates the keyspace for the Cassandra store.
    async fn create_keyspace(&self) -> Result<()> {
        let query = format!(
            "CREATE KEYSPACE IF NOT EXISTS pfacts \
                WITH REPLICATION = {{'class' : 'NetworkTopologyStrategy', 'datacenter1' : {}}};",
            self.replica_count
        );
        let params = QueryParamsBuilder::new()
            .consistency(self.write_consistency().await)
            .finalize();
        self.session
            .write()
            .await
            .query_with_params(query, params)
            .context("Query failed")?;
        Ok(())
    }

    /// Changes the system_auth keyspace to be highly available.
    // async fn change_system_auth_replication_factor(&self) -> Result<()> {
    //     let query = format!(
    //         r#"
    //      ALTER KEYSPACE "system_auth"
    //       WITH REPLICATION = {{
    //         'class' : 'SimpleStrategy',
    //         'replication_factor' : {}
    //       }};"#,
    //         self.replica_count
    //     );
    //     let params = QueryParamsBuilder::new().finalize();
    //     self.session
    //         .write()
    //         .await
    //         .query_with_params(query, params)
    //         .context("Query failed")?;
    //     Ok(())
    // }

    /// Creates the table to store facts.
    async fn create_table(&self) -> Result<()> {
        let params = QueryParamsBuilder::new()
            .consistency(self.write_consistency().await)
            .finalize();

        const CREATE: &'static str = "CREATE TABLE IF NOT EXISTS pfacts.facts \
                (key uuid PRIMARY KEY, fact varchar, kind varchar);";
        self.session
            .write()
            .await
            .query_with_params(CREATE, params)
            .context("Query failed")?;

        // It's OK if this fails...
        const TRUNCATE: &'static str = "TRUNCATE pfacts.facts;";
        let params = QueryParamsBuilder::new().finalize();
        let _ = self
            .session
            .write()
            .await
            .query_with_params(TRUNCATE, params);

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
