use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVersion {
    pub workflow_id: String,
    pub version: u32,
    pub n8n_json: String,
    pub is_canary: bool,
    pub is_latest: bool,
    pub created_at: i64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployRequest {
    pub workflow_id: String,
    pub version: u32,
    pub is_canary: bool,
    pub traffic_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResponse {
    pub success: bool,
    pub version: u32,
    pub message: String,
    pub canary_url: Option<String>,
}

pub struct AutoDeployer {
    deployments: Arc<RwLock<Vec<DeploymentRecord>>>,
    max_versions: u32,
}

#[derive(Debug, Clone)]
struct DeploymentRecord {
    workflow_id: String,
    version: u32,
    is_canary: bool,
    traffic_percent: f32,
    deployed_at: i64,
    active: bool,
}

impl AutoDeployer {
    pub fn new(max_versions: u32) -> Self {
        Self {
            deployments: Arc::new(RwLock::new(Vec::new())),
            max_versions,
        }
    }

    pub async fn create_version(&self, req: DeployRequest) -> DeployResponse {
        let mut deployments = self.deployments.write().await;
        
        let old_count = deployments.iter()
            .filter(|d| d.workflow_id == req.workflow_id && d.active)
            .count() as u32;
        
        if old_count >= self.max_versions {
            return DeployResponse {
                success: false,
                version: 0,
                message: format!("Max versions ({}) reached", self.max_versions),
                canary_url: None,
            };
        }

        let record = DeploymentRecord {
            workflow_id: req.workflow_id.clone(),
            version: req.version,
            is_canary: req.is_canary,
            traffic_percent: req.traffic_percent,
            deployed_at: Utc::now().timestamp(),
            active: true,
        };

        deployments.push(record);

        DeployResponse {
            success: true,
            version: req.version,
            message: format!("Version {} deployed", req.version),
            canary_url: if req.is_canary {
                Some(format!("/workflow/{}?version={}&canary=true", req.workflow_id, req.version))
            } else {
                None
            },
        }
    }

    pub async fn set_canary_traffic(&self, workflow_id: &str, version: u32, percent: f32) -> bool {
        let mut deployments = self.deployments.write().await;
        
        for d in deployments.iter_mut() {
            if d.workflow_id == workflow_id && d.version == version {
                d.traffic_percent = percent;
                return true;
            }
        }
        
        false
    }

    pub async fn promote_canary(&self, workflow_id: &str, version: u32) -> bool {
        let mut deployments = self.deployments.write().await;
        
        for d in deployments.iter_mut() {
            if d.workflow_id == workflow_id && d.version == version && d.is_canary {
                d.is_canary = false;
                return true;
            }
        }
        
        false
    }

    pub async fn rollback(&self, workflow_id: &str) -> Option<u32> {
        let mut deployments = self.deployments.write().await;
        
        let idx = deployments.iter()
            .rposition(|d| d.workflow_id == workflow_id && d.active && !d.is_canary)?;
        
        deployments[idx].active = false;
        
        let prev_idx = deployments.iter()
            .rposition(|d| d.workflow_id == workflow_id && d.active);
        
        if let Some(prev) = prev_idx {
            deployments[prev].active = true;
            Some(deployments[prev].version)
        } else {
            None
        }
    }

    pub async fn get_active_versions(&self, workflow_id: &str) -> Vec<VersionInfo> {
        let deployments = self.deployments.read().await;
        
        deployments.iter()
            .filter(|d| d.workflow_id == workflow_id && d.active)
            .map(|d| VersionInfo {
                version: d.version,
                is_canary: d.is_canary,
                traffic_percent: d.traffic_percent,
                deployed_at: d.deployed_at,
            })
            .collect()
    }

    pub async fn get_deployment_stats(&self) -> DeploymentStats {
        let deployments = self.deployments.read().await;
        let active = deployments.iter().filter(|d| d.active).count();
        let canary = deployments.iter().filter(|d| d.is_canary && d.active).count();
        
        DeploymentStats {
            total_deployments: deployments.len(),
            active_workflows: active,
            canary_deployments: canary,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: u32,
    pub is_canary: bool,
    pub traffic_percent: f32,
    pub deployed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeploymentStats {
    pub total_deployments: usize,
    pub active_workflows: usize,
    pub canary_deployments: usize,
}

pub async fn start_deployer_server(port: &str) -> anyhow::Result<()> {
    use axum::{Router, routing::{get, post}, extract::{Path, State, Json}, response::IntoResponse};
    use std::sync::Arc;

    #[derive(Clone)]
    struct AppState {
        deployer: Arc<AutoDeployer>,
    }

    #[derive(Deserialize)]
    struct CreateVersionRequest {
        workflow_id: String,
        n8n_json: String,
        is_canary: Option<bool>,
    }

    async fn create_version(
        State(state): State<AppState>,
        Json(req): Json<CreateVersionRequest>,
    ) -> impl IntoResponse {
        let version = (rand_version() % 100) + 1;
        let result = state.deployer.create_version(DeployRequest {
            workflow_id: req.workflow_id,
            version,
            is_canary: req.is_canary.unwrap_or(false),
            traffic_percent: 5.0,
        }).await;
        
        (axum::http::StatusCode::OK, serde_json::to_string(&result).unwrap())
    }

    async fn get_versions(
        State(state): State<AppState>,
        Path(workflow_id): Path<String>,
    ) -> impl IntoResponse {
        let versions = state.deployer.get_active_versions(&workflow_id).await;
        (axum::http::StatusCode::OK, serde_json::to_string(&versions).unwrap())
    }

    async fn rollback(
        State(state): State<AppState>,
        Path(workflow_id): Path<String>,
    ) -> impl IntoResponse {
        match state.deployer.rollback(&workflow_id).await {
            Some(v) => (axum::http::StatusCode::OK, format!("Rolled back to version {}", v)),
            None => (axum::http::StatusCode::NOT_FOUND, "No previous version".to_string()),
        }
    }

    async fn stats(State(state): State<AppState>) -> impl IntoResponse {
        let stats = state.deployer.get_deployment_stats().await;
        (axum::http::StatusCode::OK, serde_json::to_string(&stats).unwrap())
    }

    fn rand_version() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 100) as u32
    }

    let deployer = Arc::new(AutoDeployer::new(10));
    let state = AppState { deployer };

    let app = Router::new()
        .route("/versions", post(create_version))
        .route("/versions/:workflow_id", get(get_versions))
        .route("/rollback/:workflow_id", post(rollback))
        .route("/stats", get(stats))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Auto-deployer on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_version() {
        let deployer = AutoDeployer::new(5);
        let result = deployer.create_version(DeployRequest {
            workflow_id: "test-wf".to_string(),
            version: 1,
            is_canary: true,
            traffic_percent: 5.0,
        }).await;
        
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_rollback() {
        let deployer = AutoDeployer::new(5);
        deployer.create_version(DeployRequest {
            workflow_id: "test-wf".to_string(),
            version: 1,
            is_canary: false,
            traffic_percent: 100.0,
        }).await;
        
        deployer.create_version(DeployRequest {
            workflow_id: "test-wf".to_string(),
            version: 2,
            is_canary: true,
            traffic_percent: 5.0,
        }).await;
        
        let result = deployer.rollback("test-wf").await;
        assert!(result.is_some());
    }
}