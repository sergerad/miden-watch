mod action;
mod app;
mod event;
mod rpc;
mod store;
mod sync;
mod types;
mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use miden_client::rpc::Endpoint;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::{Mutex, mpsc};

use crate::app::App;
use crate::event::EventHandler;
use crate::types::StartFrom;

#[derive(Parser)]
#[command(name = "miden-watch", about = "Miden chain explorer TUI")]
struct Cli {
    /// Miden node RPC URL (e.g. https://rpc.testnet.miden.io)
    #[arg(
        long = "url",
        env = "MIDEN_NODE_URL",
        default_value = "http://localhost:57291"
    )]
    url: String,

    /// Where to start syncing from: 'tip' (default), 'genesis', or a block number
    #[arg(long, default_value = "tip")]
    from: StartFrom,

    /// Path to SQLite database file (default: ~/.miden-watch/data.db)
    #[arg(long, default_value = None)]
    db_path: Option<PathBuf>,
}

fn parse_endpoint(url: &str) -> Result<Endpoint> {
    // Parse URL like "http://localhost:57291" or "https://rpc.testnet.miden.io"
    let url = url::Url::parse(url)?;
    let protocol = url.scheme().to_string();
    let host = url
        .host_str()
        .ok_or_else(|| color_eyre::eyre::eyre!("URL must have a host"))?
        .to_string();
    let port = url.port();
    Ok(Endpoint::new(protocol, host, port))
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Parse endpoint
    let endpoint = parse_endpoint(&cli.url)?;

    // Set up database path
    let db_path = cli.db_path.unwrap_or_else(|| {
        let mut path = dirs_default();
        path.push("data.db");
        path
    });

    let store = store::Store::new(&db_path)?;
    let store = Arc::new(Mutex::new(store));

    // Set up action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    // Set up app
    let mut app = App::new(action_tx.clone());

    // Set up terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // Spawn sync task
    let sync_store = store.clone();
    let sync_action_tx = action_tx.clone();
    let sync_start_from = cli.from;
    tokio::spawn(async move {
        if let Err(e) = sync::run_sync(
            endpoint,
            sync_start_from,
            sync_store,
            sync_action_tx.clone(),
        )
        .await
        {
            let _ = sync_action_tx.send(action::Action::SyncError(format!("Sync failed: {e}")));
        }
    });

    // Set up event handler
    let mut events = EventHandler::new(Duration::from_millis(250));

    // Main loop
    loop {
        // Render
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        // Handle events and actions
        tokio::select! {
            Some(event) = events.next() => {
                if let Some(action) = app.handle_event(event) {
                    app.update(action);
                }
            }
            Some(action) = action_rx.recv() => {
                app.update(action);
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn dirs_default() -> PathBuf {
    if let Some(home) = home_dir() {
        let mut path = home;
        path.push(".miden-watch");
        path
    } else {
        PathBuf::from(".miden-watch")
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
