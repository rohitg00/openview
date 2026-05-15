use anyhow::Result;
use clap::{Parser, Subcommand};
use openview_core::{built_in_worker_catalog, ConnectionGraph, OpenViewRuntime, ResourceKind};
use openview_worker::{
    approval_worker_manifest, runtime_manifest_json, shell_sandbox_worker_manifest,
};

#[derive(Debug, Parser)]
#[command(name = "openview")]
#[command(about = "Rust-first agent-worker orchestration control plane")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the built-in worker catalog as JSON.
    Catalog,
    /// Run a local approval-gated orchestration demo.
    Demo,
    /// Print an individual worker manifest.
    Manifest { worker: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Catalog => {
            println!(
                "{}",
                serde_json::to_string_pretty(&built_in_worker_catalog())?
            );
        }
        Command::Demo => {
            let mut runtime = OpenViewRuntime::default();
            runtime.register_worker(approval_worker_manifest())?;
            runtime.register_worker(shell_sandbox_worker_manifest())?;
            let run = runtime.start_run("inspect and run a scoped command", ["shell.sandbox"])?;
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Command::Manifest { worker } => {
            let manifest = match worker.as_str() {
                "approval.gate" => approval_worker_manifest(),
                "shell.sandbox" => shell_sandbox_worker_manifest(),
                other => built_in_worker_catalog()
                    .into_iter()
                    .find(|candidate| candidate.name == other)
                    .ok_or_else(|| anyhow::anyhow!("unknown worker: {other}"))?,
            };
            println!("{}", runtime_manifest_json(&manifest)?);
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn _graph_shape_smoke() -> Result<()> {
    let mut graph = ConnectionGraph::new("demo");
    graph.shared_resource(ResourceKind::EventStream);
    Ok(())
}
