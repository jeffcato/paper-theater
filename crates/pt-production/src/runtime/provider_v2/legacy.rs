//! Explicit aliases preserve legacy evidence without inventing v2 attempt lineage.
use super::*;
use crate::runtime::broker::AccountingImport;
use std::collections::BTreeMap;
fn aliases(tx: &Connection) -> Result<BTreeMap<String, c::ProviderQuotaScope>> {
    tx.prepare("SELECT legacy_scope,record FROM provider_v2_aliases")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .map(|r| {
            let (k, v) = r?;
            Ok((k, serde_json::from_str(&v)?))
        })
        .collect()
}
fn applies(scope: &c::ProviderQuotaScope, tuple: &c::ProviderQuotaScope) -> bool {
    let c::ProviderQuotaScope::Model {
        provider,
        project_id,
        model,
    } = tuple
    else {
        return false;
    };
    match scope {
        c::ProviderQuotaScope::Provider { provider: p } => p == provider,
        c::ProviderQuotaScope::Project {
            provider: p,
            project_id: j,
        } => p == provider && j == project_id,
        c::ProviderQuotaScope::Model {
            provider: p,
            project_id: j,
            model: m,
        } => p == provider && j == project_id && m == model,
    }
}
fn imports(tx: &Connection) -> Result<Vec<AccountingImport>> {
    tx.prepare("SELECT record FROM accounting_imports")?
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|r| Ok(serde_json::from_str(&r?)?))
        .collect()
}
fn costs(r: &AccountingImport) -> Result<c::QuotaAmounts> {
    let n = |v: i64| u64::try_from(v).map_err(|_| invalid("negative inherited accounting"));
    Ok(c::QuotaAmounts {
        money_microusd: n(r.money)?,
        requests: n(r.requests)?,
        tokens: n(r.tokens)?,
        repairs: n(r.repairs)?,
    })
}
struct Limit {
    scope: String,
    start: u64,
    reset: u64,
    retry: u64,
    capacity: c::QuotaAmounts,
}
fn limits(tx: &Connection) -> Result<Vec<Limit>> {
    Ok(tx
        .prepare(
            "SELECT scope,window_start,reset_at,retry_at,money,requests,tokens,repairs FROM limits",
        )?
        .query_map([], |r| {
            Ok(Limit {
                scope: r.get(0)?,
                start: r.get(1)?,
                reset: r.get(2)?,
                retry: r.get(3)?,
                capacity: c::QuotaAmounts {
                    money_microusd: r.get(4)?,
                    requests: r.get(5)?,
                    tokens: r.get(6)?,
                    repairs: r.get(7)?,
                },
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn attempts(tx: &Connection) -> Result<Vec<(String, u64, c::QuotaAmounts)>> {
    Ok(tx
        .prepare("SELECT scope,created,money,requests,tokens,repair FROM attempts")?
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                c::QuotaAmounts {
                    money_microusd: r.get(2)?,
                    requests: r.get(3)?,
                    tokens: r.get(4)?,
                    repairs: r.get(5)?,
                },
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn nonzero(a: &c::QuotaAmounts) -> bool {
    a.money_microusd > 0 || a.requests > 0 || a.tokens > 0 || a.repairs > 0
}
pub(super) fn bind(tx: &Connection, legacy: &str, scope: &c::ProviderQuotaScope) -> Result<()> {
    let c::ProviderQuotaScope::Model {
        provider,
        project_id,
        model,
    } = scope
    else {
        return Err(invalid(
            "legacy aliases require an explicit complete model tuple",
        ));
    };
    for s in [provider, project_id, model] {
        id(s)?;
    }
    if legacy.is_empty() || legacy.len() > 256 || legacy.chars().any(char::is_control) {
        return Err(invalid("invalid legacy scope"));
    }
    let encoded = serde_json::to_string(scope)?;
    if let Some(old) = tx
        .query_row(
            "SELECT record FROM provider_v2_aliases WHERE legacy_scope=?1",
            [legacy],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        if old != encoded {
            return fail("accounting_conflict", "legacy alias is immutable");
        }
    } else {
        tx.execute(
            "INSERT INTO provider_v2_aliases VALUES(?1,?2)",
            params![legacy, encoded],
        )?;
    }
    sync(tx)
}
pub(super) fn sync(tx: &Connection) -> Result<()> {
    let aliases = aliases(tx)?;
    let require = |scope: &str| {
        if aliases.contains_key(scope) {
            Ok(())
        } else {
            fail(
                "accounting_unbound",
                "explicit legacy scope reconciliation is required",
            )
        }
    };
    for (scope, _, used) in attempts(tx)? {
        if nonzero(&used) {
            require(&scope)?;
        }
    }
    for r in imports(tx)? {
        if nonzero(&costs(&r)?) || r.retry_at > 0 {
            require(&r.scope)?;
        }
    }
    for old in limits(tx)? {
        require(&old.scope)?;
        let scope = aliases[&old.scope].clone();
        let mut w = Window {
            id: format!(
                "legacy-{}",
                digest(format!("{}:{}:{}", scope.key(), old.start, old.reset).as_bytes())
            ),
            scope,
            start_at_ms: old.start,
            reset_at_ms: old.reset,
            capacity: old.capacity,
        };
        let records = tx
            .prepare("SELECT record FROM provider_v2_windows WHERE scope_key=?1")?
            .query_map([w.scope.key()], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for raw in records {
            let existing: Window = serde_json::from_str(&raw)?;
            if existing.start_at_ms == w.start_at_ms && existing.reset_at_ms == w.reset_at_ms {
                w.id = existing.id;
                w.capacity.money_microusd = w
                    .capacity
                    .money_microusd
                    .min(existing.capacity.money_microusd);
                w.capacity.requests = w.capacity.requests.min(existing.capacity.requests);
                w.capacity.tokens = w.capacity.tokens.min(existing.capacity.tokens);
                w.capacity.repairs = w.capacity.repairs.min(existing.capacity.repairs);
                break;
            }
        }
        tx.execute("INSERT INTO provider_v2_windows(id,scope_key,record,retry_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET record=excluded.record,retry_at=MAX(retry_at,excluded.retry_at)",params![w.id,w.scope.key(),serde_json::to_string(&w)?,old.retry])?;
    }
    Ok(())
}
pub(super) fn snapshot(tx: &Connection, w: &Window) -> Result<(c::QuotaAmounts, u64)> {
    let aliases = aliases(tx)?;
    let mut total = zero();
    let mut retry = 0;
    for (scope, created, used) in attempts(tx)? {
        if created >= w.start_at_ms
            && created < w.reset_at_ms
            && aliases
                .get(&scope)
                .is_some_and(|tuple| applies(&w.scope, tuple))
        {
            total = add(&total, &used)?;
        }
    }
    for r in imports(tx)? {
        if aliases
            .get(&r.scope)
            .is_some_and(|tuple| applies(&w.scope, tuple))
        {
            if r.window_start < (w.reset_at_ms as i64) && r.reset_at > (w.start_at_ms as i64) {
                total = add(&total, &costs(&r)?)?;
            }
            retry = retry
                .max(u64::try_from(r.retry_at).map_err(|_| invalid("negative inherited retry"))?);
        }
    }
    for limit in limits(tx)? {
        if aliases
            .get(&limit.scope)
            .is_some_and(|tuple| applies(&w.scope, tuple))
        {
            retry = retry.max(limit.retry);
        }
    }
    Ok((total, retry))
}
