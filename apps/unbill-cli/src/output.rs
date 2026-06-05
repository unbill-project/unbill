// JSON output types and formatting helpers.
//
// Each `*Out` struct is a thin, serializable view of a domain type.
// Domain types themselves do not derive Serialize — this module owns that
// concern so the core library stays independent of serialization.

use unbill_console::model::{Bill, Device, LedgerMeta, User};
use unbill_console::service::ConflictGroup;
use unbill_console::settlement::Settlement;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

// sirno:witness:unbill-cli:begin
#[derive(serde::Serialize)]
pub struct LedgerOut {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(serde::Serialize)]
pub struct BillOut {
    pub id: String,
    pub description: String,
    pub amount_cents: i64,
    pub prev: Vec<String>,
    pub created_at_ms: i64,
    pub payers: Vec<ShareOut>,
    pub payees: Vec<ShareOut>,
}

#[derive(serde::Serialize)]
pub struct ShareOut {
    pub user_id: String,
    pub shares: u32,
}

#[derive(serde::Serialize)]
pub struct UserOut {
    pub user_id: String,
    pub display_name: String,
}

#[derive(serde::Serialize)]
pub struct DeviceOut {
    pub node_id: String,
    pub added_at_ms: i64,
}

#[derive(serde::Serialize)]
pub struct ConflictGroupOut {
    pub conflicting: Vec<BillOut>,
    pub ancestors: Vec<BillOut>,
}

#[derive(serde::Serialize)]
pub struct SettlementOut {
    pub currency: String,
    pub transactions: Vec<TransactionOut>,
}

#[derive(serde::Serialize)]
pub struct TransactionOut {
    pub from_user_id: String,
    pub to_user_id: String,
    pub amount_cents: i64,
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

pub fn ledger_out(m: &LedgerMeta) -> LedgerOut {
    LedgerOut {
        id: m.ledger_id.to_string(),
        name: m.name.clone(),
        currency: m.currency.code().to_owned(),
        created_at_ms: m.created_at.as_millis(),
        updated_at_ms: m.updated_at.as_millis(),
    }
}

pub fn bill_out(b: &Bill) -> BillOut {
    let to_share_out = |s: &unbill_console::model::Share| ShareOut {
        user_id: s.user_id.to_string(),
        shares: s.shares,
    };
    BillOut {
        id: b.id.to_string(),
        description: b.description.clone(),
        amount_cents: b.amount_cents,
        prev: b.prev.iter().map(|id| id.to_string()).collect(),
        created_at_ms: b.created_at.as_millis(),
        payers: b.payers.iter().map(to_share_out).collect(),
        payees: b.payees.iter().map(to_share_out).collect(),
    }
}

pub fn device_out(d: &Device) -> DeviceOut {
    DeviceOut {
        node_id: d.node_id.to_string(),
        added_at_ms: d.added_at.as_millis(),
    }
}

pub fn user_out(user: &User) -> UserOut {
    UserOut {
        user_id: user.user_id.to_string(),
        display_name: user.display_name.clone(),
    }
}

pub fn conflict_group_out(g: &ConflictGroup) -> ConflictGroupOut {
    ConflictGroupOut {
        conflicting: g.conflicting.iter().map(bill_out).collect(),
        ancestors: g.ancestors.iter().map(bill_out).collect(),
    }
}

pub fn settlement_out(s: &Settlement) -> SettlementOut {
    SettlementOut {
        currency: s.currency.code().to_owned(),
        transactions: s
            .transactions
            .iter()
            .map(|t| TransactionOut {
                from_user_id: t.from_user_id.to_string(),
                to_user_id: t.to_user_id.to_string(),
                amount_cents: t.amount_cents,
            })
            .collect(),
    }
}
// sirno:witness:unbill-cli:end

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// Format an integer cent value as a decimal string (e.g. 1250 → "12.50").
pub fn fmt_amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents.abs() % 100)
}

/// Parse a decimal amount string into integer cents (e.g. "12.50" → 1250).
/// Whole numbers are treated as full currency units (e.g. "12" → 1200).
pub fn parse_amount(s: &str) -> anyhow::Result<i64> {
    if let Some((whole, frac)) = s.split_once('.') {
        let whole: i64 = whole
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid amount: {s:?}"))?;
        let frac_padded = format!("{:0<2}", frac);
        let cents: i64 = frac_padded[..2]
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid amount fraction: {s:?}"))?;
        Ok(whole * 100 + cents)
    } else {
        let whole: i64 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid amount: {s:?}"))?;
        Ok(whole * 100)
    }
}

pub fn print_json<T: serde::Serialize>(v: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

pub fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        None => s,
        Some((byte_pos, _)) => &s[..byte_pos],
    }
}
