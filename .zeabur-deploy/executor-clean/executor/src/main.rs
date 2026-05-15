use clap::{Parser, Subcommand};
use compiler::parser::parse_n8n_workflow;
use executor::logging;
use executor::workflow_registry;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "executor_cli")]
#[command(about = "Workflow execution CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, default_value = "workflows.db")]
    db: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    Import {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        name: String,
    },
    Run {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        params: Option<String>,
        #[arg(short, long)]
        version: Option<u32>,
        #[arg(short, long)]
        timeout: Option<u64>,
    },
    RunShmem {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        shmem: String,
        #[arg(short, long)]
        version: Option<u32>,
        #[arg(short, long)]
        timeout: Option<u64>,
    },
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
    },
    List {
        #[arg(short, long)]
        id: Option<String>,
    },
    Versions {
        #[arg(short, long)]
        id: String,
    },
    Rollback {
        #[arg(short, long)]
        id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init_logging();
    
    let cli = Cli::parse();
    
    workflow_registry::init(&cli.db)?;
    
    match cli.command {
        Commands::Import { file, id, name } => {
            tracing::info!(workflow_id = %id, name = %name, "Importing workflow");
            let json_str = fs::read_to_string(file)?;
            let n8n_json: serde_json::Value = serde_json::from_str(&json_str)?;
            let workflow = parse_n8n_workflow(&json_str)?;
            let version = workflow_registry::register_workflow(&id, &name, &n8n_json, workflow)?;
            tracing::info!(workflow_id = %id, version = version, "Workflow imported");
            println!("Imported workflow '{}' with id: {} (version {})", name, id, version);
        }
        Commands::Run { id, params, version, timeout } => {
            tracing::info!(workflow_id = %id, version = version, "Starting workflow execution");
            let start = std::time::Instant::now();
            
            let workflow = workflow_registry::get_workflow(&id, version)
                .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", id))?;
            
            let mut executor = executor::Executor::new();
            
            if let Some(params_json) = params {
                if let Ok(params_obj) = serde_json::from_str::<serde_json::Value>(&params_json) {
                    if let Some(obj) = params_obj.as_object() {
                        for (key, val) in obj {
                            executor.env.set(key, val.clone());
                        }
                    }
                }
            }
            
            let result = executor.execute_with_timeout(&workflow, timeout)?;
            let elapsed = start.elapsed().as_millis();
            
            tracing::info!(workflow_id = %id, duration_ms = elapsed, "Workflow executed successfully");
            println!("{}", serde_json::to_string(&result)?);
        }
        Commands::RunShmem { id, shmem, version, timeout } => {
            let workflow = workflow_registry::get_workflow(&id, version)
                .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", id))?;
            
            let mut shmem_obj = executor::shmem::SharedMemory::open(&shmem)
                .map_err(|e| anyhow::anyhow!("Failed to open shared memory: {}", e))?;
            let request = shmem_obj.read_request()
                .map_err(|e| anyhow::anyhow!("Failed to read from shared memory: {}", e))?;
            
            let mut executor = executor::Executor::new();
            if let Some(params) = request.params.as_object() {
                for (key, val) in params {
                    executor.env.set(key, val.clone());
                }
            }
            
            let result = executor.execute_with_timeout(&workflow, timeout)?;
            let response = executor::shmem::ShmemResponse {
                success: true,
                result: Some(result),
                error: None,
            };
            shmem_obj.write_response(&response)
                .map_err(|e| anyhow::anyhow!("Failed to write to shared memory: {}", e))?;
            println!("Workflow executed successfully via shared memory");
        }
        Commands::Serve { addr } => {
            executor::http_server::start_server(&addr).await?;
        }
        Commands::List { id } => {
            if let Some(workflow_id) = id {
                let versions = workflow_registry::list_versions(&workflow_id);
                println!("Versions for workflow '{}':", workflow_id);
                for v in versions {
                    println!("  - {}", v);
                }
            } else {
                let workflows = workflow_registry::list_workflows();
                println!("Workflows:");
                for (id, name, version) in workflows {
                    println!("  {} (v{}): {}", id, version, name);
                }
            }
        }
        Commands::Versions { id } => {
            let versions = workflow_registry::list_versions(&id);
            println!("Versions for workflow '{}':", id);
            for v in versions {
                println!("  - {}", v);
            }
        }
        Commands::Rollback { id } => {
            match workflow_registry::rollback(&id)? {
                Some(new_version) => println!("Rolled back to version {}", new_version),
                None => println!("No previous version to rollback to"),
            }
        }
    }
    
    Ok(())
}