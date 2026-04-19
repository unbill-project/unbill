// JSON output types and formatting helpers.
//
// Each `*Out` struct is a thin, serializable view of a domain type.
// Domain types themselves do not derive Serialize — this module owns that
// concern so the core library stays independent of serialization.

use unbill_core::model::{Bill, LedgerMeta, Member, Ulid};
use unbill_core::settlement::Settlement;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

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
    pub payer_user_id: String,
    pub prev: Vec<Ulid>,
    pub created_at_ms: i64,
    pub shares: Vec<ShareOut>,
}

#[derive(serde::Serialize)]
pub struct ShareOut {
    pub user_id: String,
    pub shares: u32,
}

#[derive(serde::Serialize)]
pub struct MemberOut {
    pub user_id: String,
    pub display_name: String,
}

#[derive(serde::Serialize)]
pub struct SettlementOut {
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
    BillOut {
        id: b.id.to_string(),
        description: b.description.clone(),
        amount_cents: b.amount_cents,
        payer_user_id: b.payer_user_id.to_string(),
        prev: b.prev.clone(),
        created_at_ms: b.created_at.as_millis(),
        shares: b
            .shares
            .iter()
            .map(|s| ShareOut {
                user_id: s.user_id.to_string(),
                shares: s.shares,
            })
            .collect(),
    }
}

pub fn member_out(m: &Member) -> MemberOut {
    MemberOut {
        user_id: m.user_id.to_string(),
        display_name: m.display_name.clone(),
    }
}

pub fn settlement_out(s: &Settlement) -> SettlementOut {
    SettlementOut {
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
