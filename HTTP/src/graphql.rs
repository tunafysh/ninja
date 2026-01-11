use async_graphql::{Context, EmptySubscription, Object, Result, Schema};
use ninja::manager::ShurikenManager;
use ninja::types::ShurikenState;
use std::collections::HashMap;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn list_shurikens(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let manager = ctx.data::<ShurikenManager>()?;
        let result = manager
            .list(false)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result.right().unwrap_or_default())
    }

    async fn shuriken_states(&self, ctx: &Context<'_>) -> Result<HashMap<String, ShurikenState>> {
        let manager = ctx.data::<ShurikenManager>()?;
        let result = manager
            .list(true)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result.left().unwrap_or_default().into_iter().collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn start_shuriken(&self, ctx: &Context<'_>, name: String) -> Result<String> {
        let manager = ctx.data::<ShurikenManager>()?;
        manager
            .start(&name)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(format!("Shuriken '{}' started", name))
    }

    async fn stop_shuriken(&self, ctx: &Context<'_>, name: String) -> Result<String> {
        let manager = ctx.data::<ShurikenManager>()?;
        manager
            .stop(&name)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(format!("Shuriken '{}' stopped", name))
    }
}

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(manager: ShurikenManager) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(manager)
        .finish()
}
