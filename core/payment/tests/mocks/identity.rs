#![allow(dead_code)]

use anyhow::{anyhow, bail};
use futures::FutureExt;
use serde::de::DeserializeOwned;
use ya_client_model::NodeId;
use ya_core_model::appkey::AppKey;
use ya_core_model::identity::IdentityInfo;
use ya_framework_basic::mocks::net::MockNet;
use ya_identity::cli::{AppKeyCommand, IdentityCommand};

use ya_identity::service::Identity;
use ya_persistence::executor::DbExecutor;
use ya_service_api::{CliCtx, CommandOutput};

#[derive(Clone)]
pub struct MockIdentity {
    net: MockNet,
    name: String,
    db: DbExecutor,
}

impl MockIdentity {
    pub fn new(net: MockNet, name: &str) -> Self {
        let db = Self::create_db(&format!("{name}.identity.db")).unwrap();

        MockIdentity {
            net,
            name: name.to_string(),
            db,
        }
    }

    fn create_db(name: &str) -> anyhow::Result<DbExecutor> {
        let db = DbExecutor::in_memory(name)
            .map_err(|e| anyhow!("Failed to create in memory db [{name:?}]. Error: {e}"))?;
        Ok(db)
    }

    pub async fn bind_gsb(&self) -> anyhow::Result<()> {
        log::info!("MockIdentity ({}) - binding GSB", self.name);
        Identity::gsb(&self.db).await?;
        Ok(())
    }

    pub async fn create_identity_key(&self, name: &str) -> anyhow::Result<AppKey> {
        let identity: IdentityInfo = self
            .create_identity(name)
            .await
            .map_err(|e| anyhow!("Creating Identity: {e}"))?;
        let appkey = self
            .create_appkey(name, identity.node_id)
            .await
            .map_err(|e| anyhow!("Creating AppKey: {e}"))?;

        self.register_identity_in_net(identity.node_id);
        Ok(appkey)
    }

    fn register_identity_in_net(&self, id: NodeId) {
        // This line is temporary, until we will be able to rebind all modules to non-fixed prefix.
        // Currently, all modules must be bound under `/local/{module}` and `/public/{module}`.
        // Not doing so would break most of them.
        // For example Payment module uses fixed prefix to call market and identity modules.
        // When we will work around this problem, we will be able to instantiate many nodes in tests.
        self.net.register_node(&id, "/public");

        // Should be instead in the future:
        // self.net
        //     .register_node(&id, &format!("/{}/public/{id}", self.name));
    }

    pub async fn create_identity(&self, name: &str) -> anyhow::Result<IdentityInfo> {
        let ctx = CliCtx::default();
        let command = IdentityCommand::Create {
            no_password: true,
            alias: Some(name.to_string()),
            password: None,
            from_keystore: None,
            from_private_key: None,
        };

        parse_output_result::<IdentityInfo>(command.run_command(&ctx).boxed_local().await?)
    }
    pub async fn create_appkey(&self, name: &str, id: NodeId) -> anyhow::Result<AppKey> {
        let ctx = CliCtx::default();
        let command = AppKeyCommand::Create {
            name: name.to_string(),
            role: "manager".to_string(),
            id: Some(id.to_string()),
            allow_origins: vec![],
        };
        let _key = command.run_command(&ctx).await?;

        let output = AppKeyCommand::Show {
            name: name.to_string(),
        }
        .run_command(&ctx)
        .boxed_local()
        .await?;

        parse_output::<AppKey>(output)
    }
}

fn parse_output_result<T: DeserializeOwned>(output: CommandOutput) -> anyhow::Result<T> {
    Ok(match output {
        CommandOutput::Object(json) => serde_json::from_value::<Result<T, String>>(json)
            .map_err(|e| anyhow!("Error parsing command response: {e}"))?
            .map_err(|e| anyhow!("Command failed: {e}"))?,
        _ => bail!("Unexpected output: {output:?}"),
    })
}

fn parse_output<T: DeserializeOwned>(output: CommandOutput) -> anyhow::Result<T> {
    Ok(match output {
        CommandOutput::Object(json) => serde_json::from_value::<T>(json)
            .map_err(|e| anyhow!("Error parsing command response: {e}"))?,
        _ => bail!("Unexpected output: {output:?}"),
    })
}
