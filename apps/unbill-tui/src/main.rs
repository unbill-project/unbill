use std::io;
use std::sync::Arc;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use unbill_core::path::UNBILL_PATH;
use unbill_core::service::UnbillService;
use unbill_core::storage::FsStore;

mod app;
mod pane;
mod popup;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("warn".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let data_dir = UNBILL_PATH.ensure_data_dir()?;
    let store = Arc::new(FsStore::new(data_dir));
    let svc = UnbillService::open(store).await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app::run(&mut terminal, svc).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
