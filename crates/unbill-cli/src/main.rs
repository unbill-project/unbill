// unbill-cli: command-line frontend for UnbillService.
// All business logic lives in unbill-core; this file is pure dispatch + I/O.

use std::sync::Arc;

use anyhow::{bail, Context as _};
use clap::Parser;
use unbill_core::service::UnbillService;
use unbill_core::storage::FsStore;

mod commands;
mod output;

// ---------------------------------------------------------------------------
// CLI argument structure
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "unbill", about = "Peer-to-peer bill splitting.")]
pub struct Cli {
    /// Output results as JSON (useful for scripting and e2e tests).
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Initialize this device (generates a key if one does not exist).
    Init,
    /// Manage user identities on this device.
    Identity {
        #[command(subcommand)]
        sub: IdentityCmd,
    },
    /// Show information about this device.
    Device {
        #[command(subcommand)]
        sub: DeviceCmd,
    },
    /// Manage ledgers.
    Ledger {
        #[command(subcommand)]
        sub: LedgerCmd,
    },
    /// Manage bills.
    Bill {
        #[command(subcommand)]
        sub: BillCmd,
    },
    /// Manage members.
    Member {
        #[command(subcommand)]
        sub: MemberCmd,
    },
    /// Sync with peers.
    Sync {
        #[command(subcommand)]
        sub: SyncCmd,
    },
    /// Show net settlement for a user across all ledgers.
    Settlement { user_id: String },
}

#[derive(clap::Subcommand)]
pub enum IdentityCmd {
    /// Add a fresh user identity (new user ID + display name) to this device.
    New { display_name: String },
    /// Import an existing identity from another device via an unbill://identity/... URL.
    Import { url: String },
    /// List all identities stored on this device.
    List,
    /// Generate an unbill://identity/... URL to share a specific identity with another device.
    Share {
        #[arg(long)]
        user_id: String,
    },
}

#[derive(clap::Subcommand)]
pub enum DeviceCmd {
    /// Print this device's ID and data directory.
    Show,
    /// Remove an authorized device from a ledger.
    Remove {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        node_id: String,
    },
}

#[derive(clap::Subcommand)]
pub enum LedgerCmd {
    /// Create a new ledger with a name and ISO 4217 currency code (e.g. USD).
    Create {
        name: String,
        currency: String,
    },
    List,
    Show {
        ledger_id: String,
    },
    Delete {
        ledger_id: String,
    },
    /// Generate an unbill://join/... URL authorizing a new device to access this ledger.
    Invite {
        #[arg(long)]
        ledger_id: String,
    },
    /// Join a ledger using an unbill://join/... URL.
    Join {
        url: String,
    },
}

#[derive(clap::Subcommand)]
pub enum BillCmd {
    /// Add a bill to a ledger.
    Add {
        #[arg(long)]
        ledger_id: String,
        /// User ID of who paid.
        #[arg(long)]
        payer: String,
        /// Amount in the ledger's currency (e.g. 12.50).
        #[arg(long)]
        amount: String,
        #[arg(long)]
        description: String,
        /// Participant user IDs (equal shares). Repeat for each participant.
        #[arg(long = "participant")]
        participants: Vec<String>,
    },
    /// List all bills in a ledger.
    List {
        #[arg(long)]
        ledger_id: String,
    },
    /// Amend a bill. At least one of --amount, --description, --participant must be provided.
    Amend {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        bill_id: String,
        /// User ID of the person making this amendment.
        #[arg(long)]
        author: String,
        #[arg(long)]
        amount: Option<String>,
        #[arg(long)]
        description: Option<String>,
        /// Replace all participants (equal shares). Repeat for each.
        #[arg(long = "participant")]
        participants: Vec<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    Delete {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        bill_id: String,
    },
    Restore {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        bill_id: String,
    },
}

#[derive(clap::Subcommand)]
pub enum MemberCmd {
    List {
        #[arg(long)]
        ledger_id: String,
    },
    /// Add a member directly by user ID and display name.
    Add {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        user_id: String,
        #[arg(long)]
        name: String,
        /// User ID of the person performing this action.
        #[arg(long)]
        added_by: String,
    },
    /// Remove a member from a ledger.
    Remove {
        #[arg(long)]
        ledger_id: String,
        #[arg(long)]
        user_id: String,
    },
}

#[derive(clap::Subcommand)]
pub enum SyncCmd {
    /// Open the endpoint and wait for incoming sync connections.
    Daemon,
    /// Dial a specific peer by NodeId and sync all shared ledgers.
    Once { peer_node_id: String },
    /// Show sync status.
    Status,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let json = cli.json;

    let data_dir = if let Ok(p) = std::env::var("UNBILL_DATA_DIR") {
        std::path::PathBuf::from(p)
    } else {
        dirs::data_dir()
            .context("could not resolve data directory")?
            .join("unbill")
    };

    let store = Arc::new(FsStore::new(data_dir.clone()));
    let svc = UnbillService::open(store).await?;

    match cli.command {
        Command::Init => commands::init(&svc, json).await,
        Command::Identity { sub } => match sub {
            IdentityCmd::New { display_name } => {
                commands::identity_new(&svc, display_name, json).await
            }
            IdentityCmd::Import { url } => bail!("identity import is available from M3: {url}"),
            IdentityCmd::List => commands::identity_list(&svc, json).await,
            IdentityCmd::Share { user_id } => commands::identity_share(&svc, &user_id, json).await,
        },
        Command::Device { sub } => match sub {
            DeviceCmd::Show => commands::device_show(&svc, &data_dir, json).await,
            DeviceCmd::Remove { ledger_id, node_id } => {
                commands::device_remove(&svc, &ledger_id, &node_id).await
            }
        },
        Command::Ledger { sub } => match sub {
            LedgerCmd::Create { name, currency } => {
                commands::ledger_create(&svc, name, currency, json).await
            }
            LedgerCmd::List => commands::ledger_list(&svc, json).await,
            LedgerCmd::Show { ledger_id } => commands::ledger_show(&svc, &ledger_id, json).await,
            LedgerCmd::Delete { ledger_id } => commands::ledger_delete(&svc, &ledger_id).await,
            LedgerCmd::Invite { ledger_id } => {
                commands::ledger_invite(&svc, &ledger_id, json).await
            }
            LedgerCmd::Join { .. } => bail!("ledger join is available from M3"),
        },
        Command::Bill { sub } => match sub {
            BillCmd::Add {
                ledger_id,
                payer,
                amount,
                description,
                participants,
            } => {
                commands::bill_add(
                    &svc,
                    &ledger_id,
                    &payer,
                    &amount,
                    description,
                    participants,
                    json,
                )
                .await
            }
            BillCmd::List { ledger_id } => commands::bill_list(&svc, &ledger_id, json).await,
            BillCmd::Amend {
                ledger_id,
                bill_id,
                author,
                amount,
                description,
                participants,
                reason,
            } => {
                commands::bill_amend(
                    &svc,
                    &ledger_id,
                    &bill_id,
                    &author,
                    amount.as_deref(),
                    description,
                    participants,
                    reason,
                    json,
                )
                .await
            }
            BillCmd::Delete { ledger_id, bill_id } => {
                commands::bill_delete(&svc, &ledger_id, &bill_id).await
            }
            BillCmd::Restore { ledger_id, bill_id } => {
                commands::bill_restore(&svc, &ledger_id, &bill_id).await
            }
        },
        Command::Member { sub } => match sub {
            MemberCmd::List { ledger_id } => commands::member_list(&svc, &ledger_id, json).await,
            MemberCmd::Add {
                ledger_id,
                user_id,
                name,
                added_by,
            } => commands::member_add(&svc, &ledger_id, &user_id, name, &added_by).await,
            MemberCmd::Remove { ledger_id, user_id } => {
                commands::member_remove(&svc, &ledger_id, &user_id).await
            }
        },
        Command::Sync { sub } => match sub {
            SyncCmd::Once { peer_node_id } => commands::sync_once(&svc, &peer_node_id).await,
            SyncCmd::Daemon => commands::sync_daemon(&svc).await,
            SyncCmd::Status => bail!("sync status is available from M3"),
        },
        Command::Settlement { user_id } => commands::settlement(&svc, &user_id, json).await,
    }
}
