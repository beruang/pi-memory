use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "agent-memory")]
#[command(about = "Cross-session agent memory daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the memory daemon, API, and workers
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
        #[arg(long)]
        config: Option<String>,
    },
    /// Run the MCP server over stdio
    Mcp,
    /// Run PostgreSQL migrations
    Migrate {
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Perform human-facing memory search
    Search {
        query: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        project_id: Option<String>,
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Run task-aware recall
    Recall {
        #[arg(long)]
        task: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        project_id: Option<String>,
    },
    /// Run session consolidation
    Consolidate {
        #[arg(long)]
        session: String,
        #[arg(long)]
        project_id: Option<String>,
    },
    /// Display one memory item with provenance
    Inspect { observation_id: String },
    /// Start the local memory review interface
    Ui {
        #[arg(long, default_value = "3000")]
        port: u16,
        #[arg(long)]
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, config } => {
            commands::serve(port, config).await;
        }
        Commands::Mcp => {
            commands::mcp().await;
        }
        Commands::Migrate { database_url } => {
            commands::migrate_cmd(database_url).await;
        }
        Commands::Search {
            query,
            scope,
            project_id,
            limit,
        } => {
            commands::search(query, scope, project_id, limit).await;
        }
        Commands::Recall {
            task,
            scope,
            project_id,
        } => {
            commands::recall(task, scope, project_id).await;
        }
        Commands::Consolidate {
            session,
            project_id,
        } => {
            commands::consolidate(session, project_id).await;
        }
        Commands::Inspect { observation_id } => {
            commands::inspect(observation_id).await;
        }
        Commands::Ui { port, config } => {
            commands::ui(port, config).await;
        }
    }
}
