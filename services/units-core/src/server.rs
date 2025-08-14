use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;

use units_runtime::MockRuntime;
use units_storage_impl::ConsolidatedUnitsStorage;

use crate::config::Config;
use crate::service::UnitsService;

pub struct UnitsServer {
    service: UnitsService,
}

impl UnitsServer {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize storage based on config
        let storage = match config.storage.storage_type.as_str() {
            "memory" => {
                Arc::new(ConsolidatedUnitsStorage::create())
            }
            "file" => {
                // Would initialize file-based storage here
                Arc::new(ConsolidatedUnitsStorage::create())
            }
            _ => {
                anyhow::bail!("Unsupported storage type: {}", config.storage.storage_type);
            }
        };

        // Initialize runtime (using mock for now)
        let runtime: Arc<dyn units_runtime::Runtime + Send + Sync> = Arc::new(MockRuntime::new());

        // Create service
        let service = UnitsService::new(storage, runtime, config);

        Ok(Self { service })
    }

    pub async fn start_json_rpc_server(
        &self,
        addr: SocketAddr,
    ) -> Result<impl std::future::Future<Output = ()>> {
        use crate::json_rpc::JsonRpcServerImpl;
        
        let server_impl = JsonRpcServerImpl::new(self.service.clone());
        let server = server_impl.start(addr).await?;
        
        Ok(async move {
            server.await
        })
    }
}