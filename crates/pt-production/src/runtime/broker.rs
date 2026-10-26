use super::store::*;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    pub identity: String,
    pub recipe: String,
    pub provider: String,
    pub project: String,
    pub model: String,
    pub request_hash: String,
    pub payload: Value,
    pub money: i64,
    pub requests: i64,
    pub tokens: i64,
    pub repair: bool,
    pub timeout_ms: u64,
}
impl Operation {
    fn scope(&self) -> String {
        format!("{}/{}/{}", self.provider, self.project, self.model)
    }
    fn validate(&self) -> Result<()> {
        for s in [
            &self.identity,
            &self.recipe,
            &self.provider,
            &self.project,
            &self.model,
        ] {
            if s.is_empty()
                || s.len() > 128
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
            {
                return fail("invalid_operation", "invalid operation identifier");
            }
        }
        if self.money < 0
            || self.tokens < 0
            || self.requests < 1
            || self.timeout_ms == 0
            || self.timeout_ms > 300_000
        {
            return fail("invalid_operation", "invalid reservation/timeout");
        }
        let bytes = serde_json::to_vec(&self.payload)?;
        if bytes.len() > 1024 * 1024 || digest(&bytes) != self.request_hash {
            return fail("invalid_operation", "bounded payload digest mismatch");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Limits {
    pub money: i64,
    pub requests: i64,
    pub tokens: i64,
    pub repairs: i64,
    pub window_start: i64,
    pub reset_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Receipt {
    /// completed, rejected or ambiguous. Unknown charges retain the reservation.
    pub outcome: String,
    pub actual_money: Option<i64>,
    pub actual_tokens: Option<i64>,
    pub response_hash: Option<String>,
    pub job_id: Option<String>,
    pub retry_at: Option<i64>,
}
#[derive(Debug)]
pub(super) enum Dispatch {
    Authorized(String),
    Existing {
        id: String,
        state: String,
        receipt: Option<Receipt>,
    },
}
pub(super) trait Transport: Send + Sync {
    /// Receives one already reserved attempt; implementations may send exactly once.
    fn send<'a>(
        &'a self,
        attempt: &'a str,
        operation: &'a Operation,
    ) -> Pin<Box<dyn Future<Output = Result<Receipt>> + Send + 'a>>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct AccountingImport {
    pub original_hash: String,
    pub derivative_hash: String,
    pub scope: String,
    pub money: i64,
    pub requests: i64,
    pub tokens: i64,
    pub repairs: i64,
    pub window_start: i64,
    pub reset_at: i64,
    pub retry_at: i64,
    pub transformation: String,
}
impl Store {
    /// Administrative, neutral hash-bound records only. Original receipts stay external.
    pub(super) fn import_accounting(&self, record: &AccountingImport) -> Result<()> {
        if !valid_hash(&record.original_hash)
            || !valid_hash(&record.derivative_hash)
            || record.scope.is_empty()
            || [
                record.money,
                record.requests,
                record.tokens,
                record.repairs,
                record.retry_at,
            ]
            .iter()
            .any(|v| *v < 0)
        {
            return fail("invalid_accounting", "invalid neutral import");
        }
        let encoded = serde_json::to_string(record)?;
        self.transaction(|tx| {
            let prior: Option<String> = tx
                .query_row(
                    "SELECT record FROM accounting_imports WHERE original_hash=?1",
                    [&record.original_hash],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(prior) = prior {
                if prior != encoded {
                    return fail(
                        "accounting_conflict",
                        "original receipt already imported differently",
                    );
                }
                return Ok(());
            }
            tx.execute(
                "INSERT INTO accounting_imports VALUES(?1,?2,?3)",
                params![record.original_hash, record.derivative_hash, encoded],
            )?;
            tx.execute(
                "UPDATE limits SET retry_at=MAX(retry_at,?2) WHERE scope=?1",
                params![record.scope, record.retry_at],
            )?;
            Ok(())
        })
    }
    /// Requires external authorization evidence plus reconciled inherited accounting.
    /// No HTTP endpoint exposes this administrative capability.
    pub(super) fn authorize_live(&self, proof_hash: &str, cap: i64) -> Result<()> {
        if !valid_hash(proof_hash) || cap < 0 {
            return fail("invalid_authorization", "hash-bound authorization required");
        }
        self.transaction(|tx| {
            let count: i64 =
                tx.query_row("SELECT COUNT(*) FROM accounting_imports", [], |r| r.get(0))?;
            if count == 0 {
                return fail(
                    "accounting_pending",
                    "inherited accounting must be reconciled",
                );
            }
            let old: Option<(String, i64)> = tx
                .query_row(
                    "SELECT proof_hash,cap FROM authorization WHERE id=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some(old) = old {
                if old != (proof_hash.to_owned(), cap) {
                    return fail(
                        "authorization_conflict",
                        "authorization cannot reset consumption",
                    );
                }
                return Ok(());
            }
            tx.execute(
                "INSERT INTO authorization VALUES(1,?1,?2)",
                params![proof_hash, cap],
            )?;
            Ok(())
        })
    }
    pub(super) fn configure_limit(
        &self,
        provider: &str,
        project: &str,
        model: &str,
        limit: &Limits,
    ) -> Result<()> {
        if [limit.money, limit.requests, limit.tokens, limit.repairs]
            .iter()
            .any(|v| *v < 0)
            || limit.reset_at <= limit.window_start
        {
            return fail("invalid_limit", "invalid limits/window");
        }
        let scope = format!("{provider}/{project}/{model}");
        self.transaction(|tx| {
            let prior:Option<(i64,i64)>=tx.query_row("SELECT window_start,reset_at FROM limits WHERE scope=?1",[&scope],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((start,reset))=prior { if now()<reset && (start!=limit.window_start || reset!=limit.reset_at) {return fail("invalid_window","cannot reset an active quota window");} }
            tx.execute("INSERT INTO limits VALUES(?1,?2,?3,?4,?5,?6,?7,0) ON CONFLICT(scope) DO UPDATE SET money=excluded.money,requests=excluded.requests,tokens=excluded.tokens,repairs=excluded.repairs,window_start=excluded.window_start,reset_at=excluded.reset_at",params![scope,limit.money,limit.requests,limit.tokens,limit.repairs,limit.window_start,limit.reset_at])?;
            Ok(())
        })
    }
    pub(super) fn allow_recipe(&self, recipe: &str) -> Result<()> {
        if recipe.is_empty() || recipe.len() > 128 {
            return fail("invalid_recipe", "recipe name required");
        }
        self.transaction(|tx| {
            tx.execute("INSERT OR IGNORE INTO recipes VALUES(?1)", [recipe])?;
            Ok(())
        })
    }
    pub(super) fn reserve(&self, run: &str, epoch: i64, op: &Operation) -> Result<Dispatch> {
        op.validate()?;
        let descriptor_hash = digest(&serde_json::to_vec(op)?);
        self.transaction(|tx| {
            Self::active(tx,run,epoch)?;
            if let Some((id,hash,state,response))=tx.query_row("SELECT id,descriptor_hash,state,response FROM attempts WHERE run=?1 AND operation=?2",params![run,op.identity],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?))).optional()? {
                if hash!=descriptor_hash {return fail("operation_conflict","operation identity already binds another payload");}
                return Ok(Dispatch::Existing{id,state,receipt:response.map(|s|serde_json::from_str(&s)).transpose()?});
            }
            let allowed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM recipes WHERE name=?1)",[&op.recipe],|r|r.get(0))?;
            if !allowed {return fail("recipe_forbidden","operation recipe is not allowlisted");}
            let cap: Option<i64>=tx.query_row("SELECT cap FROM authorization WHERE id=1",[],|r|r.get(0)).optional()?;
            let cap=cap.ok_or_else(||Error::new("live_disabled","no product authorization"))?;
            let inherited = tx.prepare("SELECT record FROM accounting_imports")?.query_map([],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?.into_iter().map(|r|serde_json::from_str::<AccountingImport>(&r)).collect::<std::result::Result<Vec<_>,_>>()?;
            let used:i64=tx.query_row("SELECT COALESCE(SUM(money),0) FROM attempts",[],|r|r.get(0))?;
            let total=used.checked_add(op.money).and_then(|n|inherited.iter().try_fold(n,|n,r|n.checked_add(r.money))).ok_or_else(||Error::new("budget_exhausted","accounting overflow"))?;
            if total>cap {return fail("budget_exhausted","money reservation exceeds product authorization");}
            let scope=op.scope();
            let (money,requests,tokens,repairs,start,reset,cooldown):(i64,i64,i64,i64,i64,i64,i64)=tx.query_row("SELECT money,requests,tokens,repairs,window_start,reset_at,retry_at FROM limits WHERE scope=?1",[&scope],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?.ok_or_else(||Error::new("limits_unknown","provider/project/model limits required"))?;
            let learned=inherited.iter().filter(|r|r.scope==scope).map(|r|r.retry_at).max().unwrap_or(0);
            if now()<cooldown.max(learned) {return Err(Error::new("provider_wait",format!("retry_at={}",cooldown.max(learned))));}
            if now()<start || now()>=reset {return fail("limits_expired","refresh quota window without clearing consumption");}
            let mut used:(i64,i64,i64,i64)=tx.query_row("SELECT COALESCE(SUM(money),0),COALESCE(SUM(requests),0),COALESCE(SUM(tokens),0),COALESCE(SUM(repair),0) FROM attempts WHERE scope=?1 AND created>=?2 AND created<?3",params![scope,start,reset],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
            for r in inherited.iter().filter(|r|r.scope==scope && r.window_start<reset && r.reset_at>start) {used.0=used.0.saturating_add(r.money);used.1=used.1.saturating_add(r.requests);used.2=used.2.saturating_add(r.tokens);used.3=used.3.saturating_add(r.repairs);}
            for (a,b,max,code) in [(used.0,op.money,money,"budget_exhausted"),(used.1,op.requests,requests,"request_quota"),(used.2,op.tokens,tokens,"token_quota"),(used.3,i64::from(op.repair),repairs,"repair_exhausted")] {
                if a.checked_add(b).is_none_or(|n|n>max) {return Err(Error::new(code,format!("reservation unavailable; retry_at={reset}")));}
            }
            let unresolved:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM attempts WHERE scope=?1 AND request_hash=?2 AND requests>0 AND state IN ('reserved','dispatched','uncertain'))",params![scope,op.request_hash],|r|r.get(0))?;
            if unresolved{return fail("attempt_unresolved","an earlier run/operation has an unresolved matching provider request");}
            let id=digest(format!("{run}:{}",op.identity).as_bytes());
            tx.execute("INSERT INTO attempts(id,run,epoch,operation,request_hash,descriptor_hash,scope,state,money,requests,tokens,repair,created) VALUES(?1,?2,?3,?4,?5,?6,?7,'reserved',?8,?9,?10,?11,?12)",params![id,run,epoch,op.identity,op.request_hash,descriptor_hash,scope,op.money,op.requests,op.tokens,op.repair,now()])?;
            Self::emit(tx,run,"budget_reserved",&json!({"attempt_id":id,"scope":scope,"money":op.money,"requests":op.requests,"tokens":op.tokens,"repair":op.repair}))?;
            Ok(Dispatch::Authorized(id))
        })
    }
    fn mark_dispatched(&self, run: &str, epoch: i64, attempt: &str) -> Result<()> {
        self.transaction(|tx| { Self::active(tx,run,epoch)?;
            if tx.execute("UPDATE attempts SET state='dispatched' WHERE id=?1 AND run=?2 AND epoch=?3 AND state='reserved'",params![attempt,run,epoch])?!=1 {return fail("attempt_not_dispatchable","attempt already dispatched or uncertain");}
            Self::emit(tx,run,"provider_dispatched",&json!({"attempt_id":attempt}))?; Ok(())
        })
    }
    pub(super) fn record_receipt(&self, attempt: &str, receipt: &Receipt) -> Result<()> {
        if !matches!(
            receipt.outcome.as_str(),
            "completed" | "rejected" | "ambiguous"
        ) || receipt.actual_money.is_some_and(|n| n < 0)
            || receipt.actual_tokens.is_some_and(|n| n < 0)
            || receipt
                .response_hash
                .as_deref()
                .is_some_and(|s| !valid_hash(s))
            || receipt.job_id.as_ref().is_some_and(|s| s.len() > 256)
        {
            return fail("invalid_receipt", "invalid transport receipt");
        }
        let encoded = serde_json::to_string(receipt)?;
        self.transaction(|tx| {
            let (run,scope,state,old,money,tokens):(String,String,String,Option<String>,i64,i64)=tx.query_row("SELECT run,scope,state,response,money,tokens FROM attempts WHERE id=?1",[attempt],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
            if matches!(state.as_str(),"completed"|"rejected") {if old.as_deref()!=Some(&encoded) {return fail("receipt_conflict","immutable response already recorded");} return Ok(());}
            let state=if receipt.outcome=="ambiguous" {"uncertain"} else {&receipt.outcome};
            let actual=if receipt.outcome=="ambiguous"{money.max(receipt.actual_money.unwrap_or(money))}else{receipt.actual_money.unwrap_or(money)}; let actual_tokens=if receipt.outcome=="ambiguous"{tokens.max(receipt.actual_tokens.unwrap_or(tokens))}else{receipt.actual_tokens.unwrap_or(tokens)};
            tx.execute("UPDATE attempts SET state=?2,money=?3,tokens=?4,response=?5,job_id=?6 WHERE id=?1",params![attempt,state,actual,actual_tokens,encoded,receipt.job_id])?;
            if let Some(retry)=receipt.retry_at {tx.execute("UPDATE limits SET retry_at=MAX(retry_at,?2) WHERE scope=?1",params![scope,retry])?;}
            // Accounting is truthful even after cancellation. It cannot commit stage/library/publication data.
            Self::emit(tx,&run,"provider_result",&json!({"attempt_id":attempt,"outcome":receipt.outcome,"money":actual,"retry_at":receipt.retry_at}))?;
            if actual>money || actual_tokens>tokens {tx.execute("UPDATE runs SET state='failed',epoch=epoch+1,error='provider exceeded reservation' WHERE id=?1 AND state IN ('running','waiting_provider')",[&run])?;Self::emit(tx,&run,"reservation_exceeded",&json!({"attempt_id":attempt}))?;}
            Ok(())
        })
    }
    fn park_provider(&self, run: &str, epoch: i64, scope: &str, reason: &str) -> Result<()> {
        self.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let (reset, cooldown): (i64, i64) = tx.query_row(
                "SELECT reset_at,retry_at FROM limits WHERE scope=?1",
                [scope],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let mut retry = if reason == "provider_wait" {
                cooldown
            } else {
                reset
            };
            let records = tx
                .prepare("SELECT record FROM accounting_imports")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for record in records {
                let record: AccountingImport = serde_json::from_str(&record)?;
                if record.scope == scope {
                    retry = retry.max(record.retry_at);
                }
            }
            if retry <= now() {
                return fail("limits_unknown", "provider retry time is unavailable");
            }
            tx.execute(
                "UPDATE runs SET state='waiting_provider',retry_at=?2 WHERE id=?1",
                params![run, retry],
            )?;
            tx.execute(
                "UPDATE stages SET state='waiting_provider' WHERE run=?1 AND state='running'",
                [run],
            )?;
            Store::emit(
                tx,
                run,
                "provider_wait",
                &json!({"scope":scope,"reason":reason,"retry_at":retry}),
            )?;
            Ok(())
        })
    }
    pub(super) async fn dispatch(
        &self,
        run: &str,
        epoch: i64,
        op: &Operation,
        transport: &dyn Transport,
    ) -> Result<Receipt> {
        let reservation = match self.reserve(run, epoch, op) {
            Ok(value) => value,
            Err(error) => {
                if matches!(
                    error.code,
                    "provider_wait" | "request_quota" | "token_quota"
                ) {
                    self.park_provider(run, epoch, &op.scope(), error.code)?;
                }
                return Err(error);
            }
        };
        let attempt = match reservation {
            Dispatch::Authorized(id) => id,
            Dispatch::Existing {
                state,
                receipt: Some(r),
                ..
            } if state == "completed" => return Ok(r),
            Dispatch::Existing { .. } => {
                return fail(
                    "attempt_unresolved",
                    "reuse/recover existing attempt; automatic repurchase forbidden",
                );
            }
        };
        self.mark_dispatched(run, epoch, &attempt)?;
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(op.timeout_ms),
            transport.send(&attempt, op),
        )
        .await;
        let receipt = match result {
            Ok(Ok(r)) => r,
            _ => Receipt {
                outcome: "ambiguous".into(),
                actual_money: None,
                actual_tokens: None,
                response_hash: None,
                job_id: None,
                retry_at: None,
            },
        };
        self.record_receipt(&attempt, &receipt)?;
        Ok(receipt)
    }
}
