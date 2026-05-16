// Command handlers — one function per CLI subcommand.
// Each function takes the service and any parsed arguments, performs the
// operation, and prints the result. Nothing here touches storage directly.

use anyhow::anyhow;
use unbill_console::model::{
    BillId, Currency, LedgerId, NewBill, NewLedger, NewUser, NewUserName, NodeId, Share, UserId,
};
use unbill_console::service::UnbillConsole;

use crate::output::{
    bill_out, conflict_group_out, device_out, fmt_amount, ledger_out, parse_amount, print_json,
    settlement_out, truncate, user_out,
};

fn parse_ledger_id(s: &str) -> anyhow::Result<LedgerId> {
    LedgerId::from_string(s).map_err(|e| anyhow!("invalid ledger ID {s:?}: {e}"))
}

fn parse_user_id(s: &str) -> anyhow::Result<UserId> {
    UserId::from_string(s).map_err(|e| anyhow!("invalid user ID {s:?}: {e}"))
}

fn parse_bill_id(s: &str) -> anyhow::Result<BillId> {
    BillId::from_string(s).map_err(|e| anyhow!("invalid bill ID {s:?}: {e}"))
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub async fn init(svc: &UnbillConsole, json: bool) -> anyhow::Result<()> {
    let id = svc.device_id().to_string();
    if json {
        print_json(&serde_json::json!({ "device_id": id }))?;
    } else {
        println!("device ID: {id}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------

pub async fn device_show(
    svc: &UnbillConsole,
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
    svc: &UnbillConsole,
    name: String,
    currency: String,
    json: bool,
) -> anyhow::Result<()> {
    let currency = Currency::from_code(&currency)
        .ok_or_else(|| anyhow::anyhow!("unknown currency code: {currency}"))?;
    let id = svc.create_ledger(NewLedger { name, currency }).await?;
    if json {
        print_json(&serde_json::json!({ "ledger_id": id }))?;
    } else {
        println!("{id}");
    }
    Ok(())
}

pub async fn ledger_list(svc: &UnbillConsole, json: bool) -> anyhow::Result<()> {
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

pub async fn ledger_show(svc: &UnbillConsole, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let lid = parse_ledger_id(ledger_id)?;
    let ledgers = svc.list_ledgers().await?;
    let meta = ledgers
        .iter()
        .find(|m| m.ledger_id == lid)
        .ok_or_else(|| anyhow!("ledger not found: {ledger_id}"))?;
    let bills = svc.list_bills(lid).await?;
    let users = svc.list_users(lid).await?;

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

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

pub async fn bill_add(
    svc: &UnbillConsole,
    ledger_id: &str,
    payer: &str,
    amount: &str,
    description: String,
    share_users: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let lid = parse_ledger_id(ledger_id)?;
    let payer_id = parse_user_id(payer)?;
    let amount_cents = parse_amount(amount)?;
    let payees = if share_users.is_empty() {
        vec![Share {
            user_id: payer_id,
            shares: 1,
        }]
    } else {
        share_users
            .iter()
            .map(|p| {
                parse_user_id(p).map(|u| Share {
                    user_id: u,
                    shares: 1,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?
    };

    let bill_id = svc
        .add_bill(
            lid,
            NewBill {
                amount_cents,
                description,
                payers: vec![Share {
                    user_id: payer_id,
                    shares: 1,
                }],
                payees,
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

pub async fn bill_list(svc: &UnbillConsole, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let bills = svc.list_bills(parse_ledger_id(ledger_id)?).await?;
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
    svc: &UnbillConsole,
    ledger_id: &str,
    prev: Vec<String>,
    payer: &str,
    amount: &str,
    description: String,
    share_users: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let lid = parse_ledger_id(ledger_id)?;
    let prev_ids = prev
        .iter()
        .map(|p| parse_bill_id(p))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let amount_cents = parse_amount(amount)?;
    let payees = share_users
        .iter()
        .map(|p| {
            parse_user_id(p).map(|u| Share {
                user_id: u,
                shares: 1,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let bill_id = svc
        .add_bill(
            lid,
            NewBill {
                amount_cents,
                description,
                payers: vec![Share {
                    user_id: parse_user_id(payer)?,
                    shares: 1,
                }],
                payees,
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

pub async fn user_create(
    svc: &UnbillConsole,
    ledger_id: &str,
    display_name: String,
    json: bool,
) -> anyhow::Result<()> {
    let user = svc
        .create_user(parse_ledger_id(ledger_id)?, NewUserName { display_name })
        .await?;
    if json {
        print_json(&user_out(&user))?;
    } else {
        println!("user ID:  {}", user.user_id);
        println!("name:     {}", user.display_name);
    }
    Ok(())
}

pub async fn all_user_list(svc: &UnbillConsole, json: bool) -> anyhow::Result<()> {
    let users = svc.list_all_users().await?;
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

pub async fn ledger_user_add(
    svc: &UnbillConsole,
    ledger_id: &str,
    user_id: &str,
    name: String,
) -> anyhow::Result<()> {
    svc.add_user(
        parse_ledger_id(ledger_id)?,
        NewUser {
            user_id: parse_user_id(user_id)?,
            display_name: name,
        },
    )
    .await?;
    Ok(())
}

pub async fn ledger_user_list(
    svc: &UnbillConsole,
    ledger_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let users = svc.list_users(parse_ledger_id(ledger_id)?).await?;
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
    svc: &UnbillConsole,
    url: String,
    label: Option<String>,
) -> anyhow::Result<()> {
    svc.join_ledger(&url, label).await?;
    Ok(())
}

pub async fn ledger_devices(
    svc: &UnbillConsole,
    ledger_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let devices = svc.list_devices(parse_ledger_id(ledger_id)?).await?;
    if json {
        print_json(&devices.iter().map(device_out).collect::<Vec<_>>())?;
    } else {
        if devices.is_empty() {
            println!("no devices");
            return Ok(());
        }
        for d in &devices {
            println!("{}", d.node_id);
        }
    }
    Ok(())
}

pub async fn ledger_invite(svc: &UnbillConsole, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let url = svc.create_invitation(parse_ledger_id(ledger_id)?).await?;
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

pub async fn sync_once(svc: &UnbillConsole, peer_node_id: &str) -> anyhow::Result<()> {
    let peer = peer_node_id
        .parse::<NodeId>()
        .map_err(|e| anyhow!("invalid node ID {peer_node_id:?}: {e}"))?;
    svc.sync_once(peer).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Conflicts
// ---------------------------------------------------------------------------

pub async fn bill_conflicts(
    svc: &UnbillConsole,
    ledger_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let groups = svc.detect_conflicts(parse_ledger_id(ledger_id)?).await?;
    if json {
        print_json(&groups.iter().map(conflict_group_out).collect::<Vec<_>>())?;
    } else {
        if groups.is_empty() {
            println!("no conflicts");
            return Ok(());
        }
        for (i, group) in groups.iter().enumerate() {
            println!("conflict {} of {}", i + 1, groups.len());
            println!("  conflicting:");
            for b in &group.conflicting {
                println!(
                    "    {:<26}  {:>10}  {}",
                    b.id,
                    fmt_amount(b.amount_cents),
                    truncate(&b.description, 32),
                );
            }
            println!("  ancestors:");
            for b in &group.ancestors {
                println!(
                    "    {:<26}  {:>10}  {}",
                    b.id,
                    fmt_amount(b.amount_cents),
                    truncate(&b.description, 32),
                );
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

pub async fn settlement(svc: &UnbillConsole, user_id: &str, json: bool) -> anyhow::Result<()> {
    let settlements = svc
        .compute_settlement_for_user(parse_user_id(user_id)?)
        .await?;
    if json {
        let out: Vec<_> = settlements.iter().map(settlement_out).collect();
        print_json(&out)?;
    } else {
        let all_empty = settlements.iter().all(|s| s.transactions.is_empty());
        if all_empty {
            println!("all settled up");
            return Ok(());
        }
        for s in &settlements {
            for t in &s.transactions {
                println!(
                    "{}  →  {}    {} {}",
                    t.from_user_id,
                    t.to_user_id,
                    s.currency.code(),
                    fmt_amount(t.amount_cents)
                );
            }
        }
    }
    Ok(())
}
