// Command handlers — one function per CLI subcommand.
// Each function takes the service and any parsed arguments, performs the
// operation, and prints the result. Nothing here touches storage directly.

use anyhow::{anyhow, bail};
use unbill_core::model::{BillAmendment, NewBill, NewMember, NodeId, Share, Ulid};
use unbill_core::service::UnbillService;

use crate::output::{
    bill_out, fmt_amount, ledger_out, member_out, parse_amount, print_json, settlement_out,
    truncate,
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

pub async fn device_remove(
    svc: &UnbillService,
    ledger_id: &str,
    node_id: &str,
) -> anyhow::Result<()> {
    let node_id = node_id
        .parse::<NodeId>()
        .map_err(|e| anyhow!("invalid node ID {node_id:?}: {e}"))?;
    svc.remove_device(ledger_id, &node_id).await?;
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
    let members = svc.list_members(ledger_id).await?;

    if json {
        print_json(&serde_json::json!({
            "ledger": ledger_out(meta),
            "bill_count": bills.len(),
            "member_count": members.len(),
        }))?;
    } else {
        println!("ID:       {}", meta.ledger_id);
        println!("Name:     {}", meta.name);
        println!("Currency: {}", meta.currency.code());
        println!("Bills:    {}", bills.len());
        println!("Members:  {}", members.len());
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
    participants: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let payer_id = parse_ulid(payer)?;
    let amount_cents = parse_amount(amount)?;
    let shares = if participants.is_empty() {
        vec![Share {
            user_id: payer_id,
            shares: 1,
        }]
    } else {
        participants
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
        if bills.is_empty() {
            println!("no bills");
            return Ok(());
        }
        println!(
            "{:<26}  {:>10}  {:<32}  {}",
            "ID", "AMOUNT", "DESCRIPTION", "FLAGS"
        );
        for b in &bills {
            let flags = match (b.was_amended, b.is_deleted) {
                (true, true) => "amended,deleted",
                (true, false) => "amended",
                (false, true) => "deleted",
                _ => "",
            };
            println!(
                "{:<26}  {:>10}  {:<32}  {}",
                b.id,
                fmt_amount(b.amount_cents),
                truncate(&b.description, 32),
                flags
            );
        }
    }
    Ok(())
}

pub async fn bill_amend(
    svc: &UnbillService,
    ledger_id: &str,
    bill_id: &str,
    author: &str,
    amount: Option<&str>,
    description: Option<String>,
    participants: Vec<String>,
    reason: Option<String>,
    _json: bool,
) -> anyhow::Result<()> {
    let new_amount_cents = amount.map(parse_amount).transpose()?;
    let new_shares = if participants.is_empty() {
        None
    } else {
        Some(
            participants
                .iter()
                .map(|p| {
                    parse_ulid(p).map(|u| Share {
                        user_id: u,
                        shares: 1,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        )
    };

    if new_amount_cents.is_none() && description.is_none() && new_shares.is_none() {
        bail!("at least one of --amount, --description, --participant must be provided");
    }

    svc.amend_bill(
        ledger_id,
        bill_id,
        BillAmendment {
            new_amount_cents,
            new_description: description,
            new_shares,
            author_user_id: parse_ulid(author)?,
            reason,
        },
    )
    .await?;
    Ok(())
}

pub async fn bill_delete(
    svc: &UnbillService,
    ledger_id: &str,
    bill_id: &str,
) -> anyhow::Result<()> {
    svc.delete_bill(ledger_id, bill_id).await?;
    Ok(())
}

pub async fn bill_restore(
    svc: &UnbillService,
    ledger_id: &str,
    bill_id: &str,
) -> anyhow::Result<()> {
    svc.restore_bill(ledger_id, bill_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

pub async fn member_add(
    svc: &UnbillService,
    ledger_id: &str,
    user_id: &str,
    name: String,
    added_by: &str,
) -> anyhow::Result<()> {
    svc.add_member(
        ledger_id,
        NewMember {
            user_id: parse_ulid(user_id)?,
            display_name: name,
            added_by: parse_ulid(added_by)?,
        },
    )
    .await?;
    Ok(())
}

pub async fn member_remove(
    svc: &UnbillService,
    ledger_id: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    svc.remove_member(ledger_id, user_id).await?;
    Ok(())
}

pub async fn member_list(svc: &UnbillService, ledger_id: &str, json: bool) -> anyhow::Result<()> {
    let members = svc.list_members(ledger_id).await?;
    if json {
        print_json(&members.iter().map(member_out).collect::<Vec<_>>())?;
    } else {
        if members.is_empty() {
            println!("no members");
            return Ok(());
        }
        for m in &members {
            println!("{:26}  {}", m.user_id, m.display_name);
        }
    }
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
