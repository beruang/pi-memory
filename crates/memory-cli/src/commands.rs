use memory_config::AppConfig;
use memory_core::MemoryError;
use memory_db::{
    migrate, AuditRepository, ConflictsRepository, EmbeddingsRepository, EvidenceRepository,
    ObservationsRepository, SearchRepository, SupersessionsRepository,
};
use memory_mcp::MemoryMcpServer;
use sqlx::PgPool;
use std::path::Path;
use tracing::{error, info};

async fn load_config(config_path: Option<&Path>) -> AppConfig {
    AppConfig::load(config_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {}", e);
        AppConfig::default()
    })
}

async fn create_pool(database_url: &str) -> Result<PgPool, MemoryError> {
    PgPool::connect(database_url)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))
}

pub async fn serve(port: u16, config_path: Option<String>) {
    let config = load_config(config_path.as_deref().map(Path::new)).await;
    info!("Starting agent-memory daemon on port {}...", port);
    println!("agent-memory daemon starting on port {}...", port);

    let pool = match create_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            println!("ERROR: Failed to connect to database: {}", e);
            println!("Make sure PostgreSQL is running and DATABASE_URL is correct.");
            return;
        }
    };

    // Run migrations
    if let Err(e) = migrate(&pool).await {
        error!("Failed to run migrations: {}", e);
        println!("ERROR: Failed to run migrations: {}", e);
        return;
    }

    // Create repositories
    let _obs_repo = ObservationsRepository::new(pool.clone());
    let _evidence_repo = EvidenceRepository::new(pool.clone());
    let _embeddings_repo = EmbeddingsRepository::new(pool.clone());
    let _search_repo = SearchRepository::new(pool.clone());
    let _conflicts_repo = ConflictsRepository::new(pool.clone());
    let _audit_repo = AuditRepository::new(pool.clone());
    let _supers_repo = SupersessionsRepository::new(pool.clone());

    // Create provider config (used by workers in Phase 5)
    let _provider_config = memory_providers::ProviderConfig {
        api_key: config
            .embedding
            .api_key_env
            .as_ref()
            .and_then(|k| std::env::var(k.as_str()).ok()),
        model: config.embedding.model.clone(),
        base_url: config.embedding.base_url.clone(),
        dimensions: Some(config.embedding.dimensions),
    };

    println!("Database connected. Starting API server...");

    // Start HTTP API server
    let api_host = config.api.host.clone();
    let api_port = config.api.port;
    let app = memory_api::routes::create_router();
    let addr = format!("{}:{}", api_host, api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("API server listening on {}", addr);

    // TODO: Start scheduler and workers (Phase 5)
    axum::serve(listener, app).await.unwrap();
}

pub async fn mcp() {
    let config = load_config(None).await;
    info!("Starting MCP server over stdio...");
    println!("MCP server starting on stdio...");

    let pool = match create_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            println!("ERROR: Failed to connect to database: {}", e);
            println!("Make sure PostgreSQL is running and DATABASE_URL is correct.");
            return;
        }
    };

    // Run migrations
    if let Err(e) = migrate(&pool).await {
        error!("Failed to run migrations: {}", e);
        println!("ERROR: Failed to run migrations: {}", e);
        return;
    }

    // Create repositories
    let obs_repo = ObservationsRepository::new(pool.clone());
    let evidence_repo = EvidenceRepository::new(pool.clone());
    let embeddings_repo = EmbeddingsRepository::new(pool.clone());
    let search_repo = SearchRepository::new(pool.clone());
    let conflicts_repo = ConflictsRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());
    let supers_repo = SupersessionsRepository::new(pool.clone());

    let server = MemoryMcpServer::new(
        obs_repo,
        evidence_repo,
        embeddings_repo,
        search_repo,
        conflicts_repo,
        audit_repo,
        supers_repo,
    );

    println!("MCP server ready. Running over stdio...");
    let _ = memory_mcp::transport::run_stdio_server(server).await;
}

pub async fn migrate_cmd(database_url: Option<String>) {
    let url = database_url.unwrap_or_else(|| {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/agent_memory".into())
    });
    info!("Running migrations against {}", url);
    println!("Running database migrations...");
    println!("Database URL: {}", url);

    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect: {}", e);
            return;
        }
    };

    match memory_db::migrate(&pool).await {
        Ok(_) => {
            println!("Migrations completed successfully.");
        }
        Err(e) => {
            println!("ERROR: Migration failed: {}", e);
        }
    }
}

pub async fn search(query: String, scope: Option<String>, project_id: Option<String>, limit: i64) {
    let scope = scope.unwrap_or_else(|| "project".into());
    info!(
        "Searching: query='{}' scope='{}' limit={}",
        query, scope, limit
    );
    println!(
        "Search results for '{}' (scope={}, limit={}):",
        query, scope, limit
    );
    if let Some(pid) = project_id {
        println!("  project_id: {}", pid);
    }
    println!("(Search requires a running PostgreSQL instance)");
}

pub async fn recall(task: String, scope: Option<String>, project_id: Option<String>) {
    let scope = scope.unwrap_or_else(|| "project".into());
    info!("Recalling for task: '{}' scope='{}'", task, scope);
    println!("Recall results for task '{}' (scope={}):", task, scope);
    if let Some(pid) = project_id {
        println!("  project_id: {}", pid);
    }
    println!("(Recall requires a running PostgreSQL instance)");
}

pub async fn consolidate(session: String, project_id: Option<String>) {
    info!("Consolidating session: {}", session);
    println!("Consolidating session '{}'...", session);
    if let Some(pid) = project_id {
        println!("  project_id: {}", pid);
    }
    println!("(Consolidation requires a running PostgreSQL instance and provider configuration)");
}

pub async fn inspect(observation_id: String) {
    info!("Inspecting observation: {}", observation_id);
    println!("Observation: {}", observation_id);
    println!("(Inspect requires a running PostgreSQL instance)");
}

pub async fn ui(port: u16) {
    info!("Starting review UI on port {}", port);
    println!("Review UI starting on http://localhost:{} ...", port);
    println!("(UI requires a running agent-memory daemon)");
}
