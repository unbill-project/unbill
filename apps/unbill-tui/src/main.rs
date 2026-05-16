use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use unbill_asymmetric_channel::rpc::RpcAsymChannel;
use unbill_console::service::UnbillConsole;
use unbill_store_fs::UNBILL_PATH;

mod app;
mod pane;
mod popup;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("warn".parse().unwrap())
                .add_directive("iroh::socket::remote_map=error".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let socket = UNBILL_PATH.socket_path()?;
    let channel = RpcAsymChannel::connect(&socket)
        .await
        .map_err(|e| anyhow::anyhow!("cannot connect to unbill-daemon: {e}"))?;
    let svc = UnbillConsole::open(channel).await;

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
