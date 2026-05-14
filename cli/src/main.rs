use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

mod commands;

#[derive(Parser)]
#[command(name = "memflow-cli")]
#[command(about = "MemFlow Interactive CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = "http://localhost:3000")]
    url: String,

    #[arg(short, long)]
    api_key: Option<String>,

    #[arg(short, long, help = "Output as JSON")]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Execute a workflow")]
    Execute {
        workflow_id: String,
        params: Option<String>,
    },
    #[clap(about = "Create a workflow from description")]
    Create {
        description: String,
    },
    #[clap(about = "List all workflows")]
    List,
    #[clap(about = "Show execution logs")]
    Logs {
        workflow_id: Option<String>,
        limit: Option<usize>,
    },
    #[clap(about = "Run learning cycle")]
    Learn,
    #[clap(about = "Show metrics")]
    Metrics,
    #[clap(about = "Check environment and API connectivity")]
    Doctor,
    #[clap(about = "Open REPL mode")]
    Repl,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let url = env::var("MEMFLOW_API_URL").unwrap_or_else(|_| cli.url.clone());
    let api_key = cli.api_key.or_else(|| env::var("MEMFLOW_API_KEY").ok());

    let client = commands::MemFlowClient::new(&url, api_key.as_deref());
    let json_output = cli.json;

    match cli.command {
        Some(Commands::Execute { workflow_id, params }) => {
            let params: Option<serde_json::Value> = params
                .as_ref()
                .and_then(|p| serde_json::from_str(p).ok());
            let result = client.execute_workflow(&workflow_id, params).await?;
            if json_output {
                println!("{}", serde_json::to_string(&result)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Some(Commands::Create { description }) => {
            let result = client.create_workflow(&description).await?;
            if json_output {
                println!("{}", serde_json::to_string(&result)?);
            } else {
                println!(
                    "Created workflow: {}",
                    result.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("unknown")
                );
            }
        }
        Some(Commands::List) => {
            let workflows = client.list_workflows().await?;
            if json_output {
                println!("{}", serde_json::to_string(&workflows)?);
            } else if workflows.is_empty() {
                println!("No workflows found");
            } else {
                for wf in workflows {
                    println!(
                        "{} - {} (v{})",
                        wf.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                        wf.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                        wf.get("version").and_then(|v| v.as_u64()).unwrap_or(0)
                    );
                }
            }
        }
        Some(Commands::Logs { workflow_id, limit }) => {
            let limit = limit.unwrap_or(10);
            let logs = if let Some(id) = workflow_id {
                client.get_workflow_logs(&id, limit).await?
            } else {
                client.get_recent_logs(limit).await?
            };
            for log in logs {
                let status = if log.get("error").and_then(|v| v.as_str()).is_some() {
                    "ERROR"
                } else {
                    "OK"
                };
                println!(
                    "[{}] {} - {}ms - {}",
                    status,
                    log.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("?"),
                    log.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0),
                    log.get("started_at").and_then(|v| v.as_str()).unwrap_or("?")
                );
            }
        }
        Some(Commands::Learn) => {
            println!("Triggering learning cycle...");
            let result = client.trigger_learn().await?;
            println!("Learning cycle completed: {:?}", result);
        }
        Some(Commands::Metrics) => {
            let metrics = client.get_metrics().await?;
            println!("{}", serde_json::to_string_pretty(&metrics)?);
        }
        Some(Commands::Doctor) => {
            run_doctor(&client).await?;
        }
        Some(Commands::Repl) => {
            run_repl(client).await?;
        }
        None => {
            run_repl(client).await?;
        }
    }

    Ok(())
}

async fn run_repl(mut client: commands::MemFlowClient) -> Result<()> {
    use rustyline::Editor;
    let mut rl: Editor<(), rustyline::history::FileHistory> = Editor::new()?;

    rl.load_history(&dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("memflow_cli_history"))?;

    println!("MemFlow CLI v0.1.0");
    println!("Type 'help' for available commands, 'exit' to quit\n");

    loop {
        let readline = rl.readline("memflow> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                let result = execute_line(&mut client, line).await;
                match result {
                    Ok(Some(output)) => println!("{}", output),
                    Ok(None) => {}
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("\nExiting...");
                break;
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn execute_line(client: &mut commands::MemFlowClient, line: &str) -> Result<Option<String>, anyhow::Error> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = parts.get(1).map(|s| *s);

    match cmd {
        "help" => Ok(Some(
            r#"Available commands:
  execute <id> [params]   Execute a workflow
  create "description"     Create a workflow from natural language
  list                    List all workflows
  logs [id] [limit]       Show execution logs
  learn                   Trigger learning cycle
  metrics                 Show performance metrics
  doctor                  Check environment and API connectivity
  exit, quit              Exit the CLI
  help                    Show this help"#.to_string()
        )),
        "exit" | "quit" => std::process::exit(0),
        "execute" => {
            let args = args.ok_or_else(|| anyhow::anyhow!("Usage: execute <workflow_id> [params_json]"))?;
            let args_parts: Vec<&str> = args.splitn(2, ' ').collect();
            let workflow_id = args_parts[0];
            let params: Option<serde_json::Value> = args_parts.get(1)
                .and_then(|p| serde_json::from_str(p).ok());
            let result = client.execute_workflow(workflow_id, params).await?;
            Ok(Some(serde_json::to_string_pretty(&result)?))
        }
        "create" => {
            let description = args.ok_or_else(|| anyhow::anyhow!("Usage: create \"description\""))?;
            let result = client.create_workflow(description).await?;
            Ok(Some(serde_json::to_string_pretty(&result)?))
        }
        "list" => {
            let workflows = client.list_workflows().await?;
            if workflows.is_empty() {
                Ok(Some("No workflows".to_string()))
            } else {
                let output: Vec<String> = workflows.iter()
                    .map(|w| format!(
                        "{} - {}",
                        w.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                        w.get("name").and_then(|v| v.as_str()).unwrap_or("?")
                    ))
                    .collect();
                Ok(Some(output.join("\n")))
            }
        }
        "logs" => {
            let args = args.unwrap_or("");
            let args_parts: Vec<&str> = args.split_whitespace().collect();
            let limit = args_parts.get(1)
                .and_then(|l| l.parse().ok())
                .unwrap_or(10);
            let logs = if let Some(id) = args_parts.first() {
                client.get_workflow_logs(id, limit).await?
            } else {
                client.get_recent_logs(limit).await?
            };
            let output: Vec<String> = logs.iter()
                .map(|l| format!(
                    "[{}] {} - {}ms",
                    if l.get("error").is_some() { "ERR" } else { "OK" },
                    l.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("?"),
                    l.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0)
                ))
                .collect();
            Ok(Some(output.join("\n")))
        }
        "learn" => {
            client.trigger_learn().await?;
            Ok(Some("Learning cycle completed".to_string()))
        }
        "metrics" => {
            let metrics = client.get_metrics().await?;
            Ok(Some(serde_json::to_string_pretty(&metrics)?))
        }
        "doctor" => {
            run_doctor(client).await?;
            Ok(None)
        }
        _ => Err(anyhow::anyhow!("Unknown command: {}. Type 'help' for available commands.", cmd)),
    }
}

async fn run_doctor(client: &commands::MemFlowClient) -> Result<()> {
    println!("\n=== MemFlow Doctor ===\n");
    
    println!("Checking API connectivity...");
    let api_ok = match client.health_check().await {
        Ok(true) => {
            println!("  [OK] API server is accessible");
            true
        }
        Ok(false) => {
            println!("  [FAIL] API server returned error");
            false
        }
        Err(e) => {
            println!("  [FAIL] Cannot connect to API: {}", e);
            false
        }
    };
    
    if api_ok {
        println!("\nChecking API key validity...");
        match client.validate_api_key().await {
            Ok(true) => {
                println!("  [OK] API key is valid");
            }
            Ok(false) => {
                println!("  [FAIL] API key is invalid (401)");
            }
            Err(e) => {
                println!("  [WARN] Cannot verify API key: {}", e);
            }
        }
    }
    
    println!("\nChecking local dependencies...");
    println!("  [OK] CLI version: 0.1.0");
    
    let platform = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unknown"
    };
    println!("  [OK] Platform: {}", platform);
    
    println!("\nChecking configuration...");
    println!("  [OK] API URL: {}", client.get_url());
    if client.get_api_key().is_some() {
        println!("  [OK] API Key: configured");
    } else {
        println!("  [WARN] API Key: not configured (set MEMFLOW_API_KEY or --api-key)");
    }
    
    println!("\n=== Doctor check complete ===\n");
    Ok(())
}