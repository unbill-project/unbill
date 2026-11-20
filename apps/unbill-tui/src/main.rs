use std::io;
use std::sync::Arc;

build_info::build_info!(fn compiled_build_info);

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use unbill_asymmetric_channel::rpc::RpcAsymChannel;
use unbill_console::service::UnbillConsole;
use unbill_storage::UNBILL_PATH;

mod app;
mod pane;
mod popup;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!(
            "unbill-tui {}",
            unbill_build_info::BuildInfo::from(crate::compiled_build_info())
        );
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("warn".parse()?)
                .add_directive("iroh::socket::remote_map=error".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    // sirno:witness:unbill-tui:begin
    let socket = UNBILL_PATH.socket_path()?;
    let channel = RpcAsymChannel::connect(&socket)
        .await
        .map_err(|e| anyhow::anyhow!("cannot connect to unbill-daemon: {e}"))?;
    let svc = UnbillConsole::open(channel).await;

    if svc.current_account().await.is_err() {
        prompt_login(&svc).await?;
    }

    loop {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = app::run(&mut terminal, Arc::clone(&svc)).await;

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        match result? {
            app::ExitReason::Quit => break,
            app::ExitReason::SwitchAccount => {
                svc.logout().await?;
                prompt_login(&svc).await?;
            }
            app::ExitReason::ChangePassword => {
                let old = prompt("Current password")?;
                let new = prompt("New password")?;
                svc.change_password(&old, &new).await?;
                prompt_login(&svc).await?;
            }
        }
    }
    Ok(())
    // sirno:witness:unbill-tui:end
}

fn prompt(label: &str) -> Result<String> {
    use std::io::Write as _;
    print!("{label}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_owned())
}

async fn prompt_login(svc: &Arc<UnbillConsole>) -> Result<()> {
    loop {
        let action = prompt("Log in or register? [l/r]")?;
        let name = prompt("Account name")?;
        let password = prompt("Password")?;
        if action.eq_ignore_ascii_case("r") {
            svc.register_account(&name, &password).await?;
        }
        match svc.login(&name, &password).await {
            Ok(_) => return Ok(()),
            Err(error) => eprintln!("{error}"),
        }
    }
}
