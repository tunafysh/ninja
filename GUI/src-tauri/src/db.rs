// src-tauri/src/db.rs
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;
use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shuriken {
    pub id: String,
    pub name: String,
    pub binary_path: String,
    pub config_path: String,
    pub status: String, // "running" | "stopped"
    pub pid: Option<i32>,
}

pub struct Database {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl Database {
    pub async fn new() -> Result<Self, surrealdb::Error> {
        let db = Surreal::new::<RocksDb>("./registry.db").await?;
        db.use_ns("shurikens").use_db("shurikens").await?;
        Ok(Self { db })
    }

    pub async fn create_shuriken(&self, shuriken: Shuriken) -> Result<(), surrealdb::Error> {
        self.db
            .create::<Option<Shuriken>>(("shuriken", &shuriken.id))
            .content(shuriken)
            .await?;
        Ok(())
    }
    pub async fn get_shuriken(&self, id: &str) -> Result<Option<Shuriken>, surrealdb::Error> {
        self.db.select(("shurikens", id)).await
    }

    pub async fn get_all_shurikens(&self) -> Result<Vec<Shuriken>, surrealdb::Error> {
        self.db.select("shurikens").await
    }

    pub async fn update_shuriken_status(
        &self,
        id: &str,
        status: &str,
        pid: Option<i32>,
    ) -> Result<(), surrealdb::Error> {
        self.db
            .update::<Option<Shuriken>>(("shurikens", id))
            .merge(json!({
                "status": status,
                "pid": pid
            }))
            .await?;
        Ok(())
    }

    pub async fn update_shuriken(&self, shuriken: Shuriken) -> Result<(), surrealdb::Error> {
        self.db
            .update::<Option<Shuriken>>(("shurikens", &shuriken.id))
            .content(shuriken)
            .await?;
        Ok(())
    }

    pub async fn delete_shuriken(&self, id: &str) -> Result<(), surrealdb::Error> {
        self.db.delete::<Option<Shuriken>>(("shurikens", id)).await?;
        Ok(())
    }
}