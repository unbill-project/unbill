// Command handlers — one function per CLI subcommand.
// Each function takes the service and any parsed arguments, performs the
// operation, and prints the result. Nothing here touches storage directly.

use std::sync::Arc;

use anyhow::anyhow;
use unbill_core::model::{NewBill, NewUser, NodeId, Share, Ulid};
use unbill_core::service::UnbillService;

use crate::output::{
    bill_out, fmt_amount, ledger_out, parse_amount, print_json, settlement_out, truncate, user_out,
};

fn parse_ulid(s: &str) -> anyhow::Result<Ulid> {
    Ulid::from_string(s).map_err(|e| anyhow!("invalid ID {s:?}: {e}"))
}

// ---------------------------------------------------------------------------
// Init / Identity
// ---------------------------------------------------------------------------

pub async fn init(svc: &UnbillService, json: bool) -> anyhow::Result<()> {
    let id = svc.device_id().to_string();
    if json {
        print_json(&serde_json::json!({ "device_id": id }))?;
    } else {
        println!("device ID: {id}");
    }
    Ok(())
}

pub async fn identity_new(
    svc: &UnbillService,
    display_name: String,
    json: bool,
) -> anyhow::Result<()> {
    let identity = svc.add_identity(display_name).await?;
    if json {
        print_json(&serde_json::json!({
            "user_id": identity.user_id.to_string(),
            "display_name": identity.display_name,
        }))?;
    } else {
        println!("user ID:  {}", identity.user_id);
        println!("name:     {}", identity.display_name);
    }
    Ok(())
}

pub async fn identity_list(svc: &UnbillService, json: bool) -> anyhow::Result<()> {
    let identities = svc.list_identities().await?;
    if json {
        let out: Vec<_> = identities
            .iter()
            .map(|i| {
                serde_json::json!({
                    "user_id": i.user_id.to_string(),
                    "display_name": i.display_name,
                })
            })
            .collect();
        print_json(&out)?;
    } else {
        if identities.is_empty() {
            println!("no identities");
            return Ok(());
        }
        for i in &identities {
            println!("{:26}  {}", i.user_id, i.display_name);
        }
    }
    Ok(())
}

pub async fn identity_remove(svc: &UnbillService, user_id: &str) -> anyhow::Result<()> {
    let uid = parse_ulid(user_id)?;
    svc.remove_identity(uid).await?;
    println!("removed identity {user_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------

pub async fn device_show(
    svc: &UnbillService,
    data_dir: &std::path::Path,
    json: bool,
) -> anyhow::Result<()> {
    let id = svc.device_id().to_string();
    let dir = data_dir.display().to_string();
    if json {
        print_json(&serde_json::json!({ "device_id": id, "data_dir": dir }))?;
    } else {
        println!("device ID: {id}");
        println!("data dir:  {dir}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

pub async fn ledger_create(
    svc: &UnbillService,
    name: String,
    currency: String,
    json: bool,
) -> anyhow::Result<()> {
    let id = svc.create_ledger(name, currency).await?;
    if json {
        print_json(&serde_json::json!({ "ledger_id": id }))?;
    } else {
        println!("{id}");
    }
    Ok(())
}

pub async fn ledger_list(svc: &UnbillService, json: bool) -> anyhow::Result<()> {
    let ledgers = svc.list_ledgers().await?;
    if json {
        print_json(&ledgers.iter().map(ledger_out).collect::<Vec<_>>())?;
    } else {
        if ledgers.is_empty() {
            println!("no ledgers");
        }
        for m in &ledgers {
            println!("{:26}  {}  ({})", m.ledger_id, m.name, m.currency.code());
        }
    }
    Ok(())
}

pub async fn ledger_show(svc: &UnbillService, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let ledgers = svc.list_ledgers().await?;
    let meta = ledgers
        .iter()
        .find(|m| m.ledger_id.to_string() == ledger_id)
        .ok_or_else(|| anyhow!("ledger not found: {ledger_id}"))?;
    let bills = svc.list_bills(ledger_id).await?;
    let users = svc.list_users(ledger_id).await?;

    if json {
        print_json(&serde_json::json!({
            "ledger": ledger_out(meta),
            "bill_count": bills.0.len(),
            "user_count": users.len(),
        }))?;
    } else {
        println!("ID:       {}", meta.ledger_id);
        println!("Name:     {}", meta.name);
        println!("Currency: {}", meta.currency.code());
        println!("Bills:    {}", bills.0.len());
        println!("Users:    {}", users.len());
    }
    Ok(())
}

pub async fn ledger_delete(svc: &UnbillService, ledger_id: &str) -> anyhow::Result<()> {
    svc.delete_ledger(ledger_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

pub async fn bill_add(
    svc: &UnbillService,
    ledger_id: &str,
    payer: &str,
    amount: &str,
    description: String,
    share_users: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let payer_id = parse_ulid(payer)?;
    let amount_cents = parse_amount(amount)?;
    let shares = if share_users.is_empty() {
        vec![Share {
            user_id: payer_id,
            shares: 1,
        }]
    } else {
        share_users
            .iter()
            .map(|p| {
                parse_ulid(p).map(|u| Share {
                    user_id: u,
                    shares: 1,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?
    };

    let bill_id = svc
        .add_bill(
            ledger_id,
            NewBill {
                payer_user_id: payer_id,
                amount_cents,
                description,
                shares,
                prev: vec![],
            },
        )
        .await?;

    if json {
        print_json(&serde_json::json!({ "bill_id": bill_id }))?;
    } else {
        println!("{bill_id}");
    }
    Ok(())
}

pub async fn bill_list(svc: &UnbillService, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let bills = svc.list_bills(ledger_id).await?;
    if json {
        print_json(&bills.iter().map(bill_out).collect::<Vec<_>>())?;
    } else {
        if bills.0.is_empty() {
            println!("no bills");
            return Ok(());
        }
        println!("{:<26}  {:>10}  DESCRIPTION", "ID", "AMOUNT");
        for b in bills.iter() {
            println!(
                "{:<26}  {:>10}  {}",
                b.id,
                fmt_amount(b.amount_cents),
                truncate(&b.description, 32),
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn bill_amend(
    svc: &UnbillService,
    ledger_id: &str,
    prev: Vec<String>,
    payer: &str,
    amount: &str,
    description: String,
    share_users: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let prev_ids = prev
        .iter()
        .map(|p| parse_ulid(p))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let amount_cents = parse_amount(amount)?;
    let shares = share_users
        .iter()
        .map(|p| {
            parse_ulid(p).map(|u| Share {
                user_id: u,
                shares: 1,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let bill_id = svc
        .add_bill(
            ledger_id,
            NewBill {
                payer_user_id: parse_ulid(payer)?,
                amount_cents,
                description,
                shares,
                prev: prev_ids,
            },
        )
        .await?;
    if json {
        print_json(&serde_json::json!({ "bill_id": bill_id }))?;
    } else {
        println!("{bill_id}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

pub async fn user_add(
    svc: &UnbillService,
    ledger_id: &str,
    user_id: &str,
    name: String,
) -> anyhow::Result<()> {
    svc.add_user(
        ledger_id,
        NewUser {
            user_id: parse_ulid(user_id)?,
            display_name: name,
        },
    )
    .await?;
    Ok(())
}

pub async fn user_list(svc: &UnbillService, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let users = svc.list_users(ledger_id).await?;
    if json {
        print_json(&users.iter().map(user_out).collect::<Vec<_>>())?;
    } else {
        if users.is_empty() {
            println!("no users");
            return Ok(());
        }
        for user in &users {
            println!("{:26}  {}", user.user_id, user.display_name);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ledger invite / join
// ---------------------------------------------------------------------------

pub async fn ledger_join(
    svc: &Arc<UnbillService>,
    url: String,
    label: Option<String>,
) -> anyhow::Result<()> {
    svc.join_ledger(&url, label.unwrap_or_default()).await?;
    Ok(())
}

pub async fn ledger_invite(
    svc: &Arc<UnbillService>,
    ledger_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let url = svc.create_invitation(ledger_id).await?;
    if json {
        print_json(&serde_json::json!({ "url": url }))?;
    } else {
        println!("{url}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Identity share
// ---------------------------------------------------------------------------

pub async fn identity_import(svc: &Arc<UnbillService>, url: String) -> anyhow::Result<()> {
    svc.fetch_identity(&url).await?;
    Ok(())
}

pub async fn identity_share(
    svc: &Arc<UnbillService>,
    user_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let url = svc.create_identity_share(user_id).await?;
    if json {
        print_json(&serde_json::json!({ "url": url }))?;
    } else {
        println!("{url}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

pub async fn sync_once(svc: &Arc<UnbillService>, peer_node_id: &str) -> anyhow::Result<()> {
    let peer = peer_node_id
        .parse::<NodeId>()
        .map_err(|e| anyhow!("invalid node ID {peer_node_id:?}: {e}"))?;
    svc.sync_once(peer).await?;
    Ok(())
}

pub async fn sync_daemon(svc: &Arc<UnbillService>) -> anyhow::Result<()> {
    svc.accept_loop().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

pub async fn settlement(svc: &UnbillService, user_id: &str, json: bool) -> anyhow::Result<()> {
    let s = svc.compute_settlement_for_user(user_id).await?;
    if json {
        print_json(&settlement_out(&s))?;
    } else {
        if s.transactions.is_empty() {
            println!("all settled up");
            return Ok(());
        }
        for t in &s.transactions {
            println!(
                "{}  →  {}    {}",
                t.from_user_id,
                t.to_user_id,
                fmt_amount(t.amount_cents)
            );
        }
    }
    Ok(())
}
