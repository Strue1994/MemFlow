use clap::Subcommand;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    api_url: String,
    api_accessible: bool,
    api_key_valid: bool,
    docker_available: bool,
    docker_version: Option<String>,
    redis_available: bool,
    redis_version: Option<String>,
    postgres_available: bool,
    clickhouse_available: bool,
    overall_status: String,
    issues: Vec<String>,
    checks: HashMap<String, CheckResult>,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    status: String,
    message: String,
    version: Option<String>,
}

impl Default for DoctorReport {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            api_accessible: false,
            api_key_valid: false,
            docker_available: false,
            docker_version: None,
            redis_available: false,
            redis_version: None,
            postgres_available: false,
            clickhouse_available: false,
            overall_status: "unknown".to_string(),
            issues: Vec::new(),
            checks: HashMap::new(),
        }
    }
}

pub async fn run_doctor(api_url: &str, api_key: &str, output_json: bool) -> Result<DoctorReport, String> {
    let mut report = DoctorReport::default();
    report.api_url = api_url.to_string();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    // Check API URL
    match client.get(&format!("{}/health", api_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            report.api_accessible = true;
            report.checks.insert("api".to_string(), CheckResult {
                status: "ok".to_string(),
                message: "API service is accessible".to_string(),
                version: None,
            });
        }
        Ok(resp) => {
            report.issues.push(format!("API returned status {}", resp.status()));
            report.checks.insert("api".to_string(), CheckResult {
                status: "error".to_string(),
                message: format!("API returned status {}", resp.status()),
                version: None,
            });
        }
        Err(e) => {
            report.issues.push(format!("Cannot connect to API: {}", e));
            report.checks.insert("api".to_string(), CheckResult {
                status: "error".to_string(),
                message: e.to_string(),
                version: None,
            });
        }
    }

    // Check API Key
    if !api_key.is_empty() {
        let req = client.get(&format!("{}/workflows", api_url))
            .header("Authorization", format!("Bearer {}", api_key));
        
        match req.send().await {
            Ok(resp) if resp.status() != 401 => {
                report.api_key_valid = true;
                report.checks.insert("api_key".to_string(), CheckResult {
                    status: "ok".to_string(),
                    message: "API key is valid".to_string(),
                    version: None,
                });
            }
            Ok(_) | Err(_) => {
                report.api_key_valid = false;
                if !report.issues.contains(&"Invalid API key".to_string()) {
                    report.issues.push("Invalid API key".to_string());
                }
                report.checks.insert("api_key".to_string(), CheckResult {
                    status: "error".to_string(),
                    message: "API key is invalid or expired".to_string(),
                    version: None,
                });
            }
        }
    } else {
        report.checks.insert("api_key".to_string(), CheckResult {
            status: "warn".to_string(),
            message: "No API key configured".to_string(),
            version: None,
        });
    }

    // Check Docker
    match Command::new("docker").arg("--version").output() {
        Ok(output) if output.status().success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            report.docker_available = true;
            report.docker_version = Some(version.clone());
            report.checks.insert("docker".to_string(), CheckResult {
                status: "ok".to_string(),
                message: "Docker is available".to_string(),
                version: Some(version),
            });
        }
        _ => {
            report.docker_available = false;
            report.checks.insert("docker".to_string(), CheckResult {
                status: "warn".to_string(),
                message: "Docker not found (optional)".to_string(),
                version: None,
            });
        }
    }

    // Check system Docker containers are running
    if report.docker_available {
        check_services(&client, api_url, &mut report).await;
    }

    // Set overall status
    if report.issues.is_empty() {
        report.overall_status = "healthy".to_string();
    } else if report.api_accessible {
        report.overall_status = "degraded".to_string();
    } else {
        report.overall_status = "unhealthy".to_string();
    }

    Ok(report)
}

async fn check_services(client: &Client, api_url: &str, report: &mut DoctorReport) {
    // Check Redis
    match client.get(&format!("{}/health/redis", api_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            report.redis_available = true;
            report.checks.insert("redis".to_string(), CheckResult {
                status: "ok".to_string(),
                message: "Redis is connected".to_string(),
                version: None,
            });
        }
        _ => {
            report.checks.insert("redis".to_string(), CheckResult {
                status: "warn".to_string(),
                message: "Redis not accessible".to_string(),
                version: None,
            });
        }
    }

    // Check Postgres
    match client.get(&format!("{}/health/postgres", api_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            report.postgres_available = true;
            report.checks.insert("postgres".to_string(), CheckResult {
                status: "ok".to_string(),
                message: "PostgreSQL is connected".to_string(),
                version: None,
            });
        }
        _ => {
            report.checks.insert("postgres".to_string(), CheckResult {
                status: "warn".to_string(),
                message: "PostgreSQL not accessible".to_string(),
                version: None,
            });
        }
    }

    // Check ClickHouse
    match client.get(&format!("{}/health/clickhouse", api_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            report.clickhouse_available = true;
            report.checks.insert("clickhouse".to_string(), CheckResult {
                status: "ok".to_string(),
                message: "ClickHouse is connected".to_string(),
                version: None,
            });
        }
        _ => {
            report.checks.insert("clickhouse".to_string(), CheckResult {
                status: "warn".to_string(),
                message: "ClickHouse not accessible".to_string(),
                version: None,
            });
        }
    }
}

pub fn print_doctor_report(report: &DoctorReport, json_output: bool) {
    if json_output {
        println!("{}", serde_json::to_string_pretty(report).unwrap_or_default());
        return;
    }

    use colored::*;
    
    println!("\n{}", "╔════════════════════════════════════════╗".green());
    println!("{}", "║       MemFlow Doctor CLI         ║".green());
    println!("{}", "╚════════════════════════════════════════╝".green());
    
    let status_color = match report.overall_status.as_str() {
        "healthy" => Color::Green,
        "degraded" => Color::Yellow,
        _ => Color::Red,
    };
    
    println!("\nOverall Status: {}", report.overall_status.as_str().color(status_color));
    println!("API URL: {}", report.api_url);
    
    if !report.issues.is_empty() {
        println!("\n{}", "Issues Found:".red().bold());
        for issue in &report.issues {
            println!("  • {}", issue.color(Color::Red));
        }
    }
    
    println!("\n{}", "Checks:".bold());
    for (name, check) in &report.checks {
        let check_color = match check.status.as_str() {
            "ok" => Color::Green,
            "warn" => Color::Yellow,
            _ => Color::Red,
        };
        let icon = match check.status.as_str() {
            "ok" => "✓",
            "warn" => "⚠",
            _ => "✗",
        };
        
        println!("  {} [{}] {}", icon, name.color(check_color), check.message);
    }
    
    println!();
}