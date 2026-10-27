//! Persistent offline broker; production construction and real dispatch remain unavailable.
mod legacy;
mod recovery;
use super::store::*;
use pt_contracts as c;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

pub(super) trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}
struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        now() as u64
    }
}
#[derive(Clone)]
pub(super) struct Broker {
    store: Store,
    clock: Arc<dyn Clock>,
    #[cfg(test)]
    synthetic: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Binding {
    pub id: String,
    pub context: c::ProviderContext,
    pub model: String,
    pub recipe_sha256: String,
    pub kind: c::OperationKind,
    pub recovery_estimate: c::Usage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Window {
    pub id: String,
    pub scope: c::ProviderQuotaScope,
    pub start_at_ms: u64,
    pub reset_at_ms: u64,
    pub capacity: c::QuotaAmounts,
}
#[derive(Clone, Serialize, Deserialize)]
struct Unit {
    binding: String,
    operation_id: String,
    repair: Option<c::CreativeRepairIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retry: Option<String>,
}
fn zero() -> c::QuotaAmounts {
    c::QuotaAmounts {
        money_microusd: 0,
        requests: 0,
        tokens: 0,
        repairs: 0,
    }
}
fn invalid(message: impl Into<String>) -> Error {
    Error::new("provider_binding", message)
}
fn contract(e: c::ContractError) -> Error {
    invalid(e.to_string())
}
fn id(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    {
        return Err(invalid("bounded identifier required"));
    }
    Ok(())
}
fn add(a: &c::QuotaAmounts, b: &c::QuotaAmounts) -> Result<c::QuotaAmounts> {
    let sum = |a: u64, b: u64| {
        a.checked_add(b)
            .filter(|v| *v <= c::MAX_SAFE_INTEGER)
            .ok_or_else(|| Error::new("accounting_overflow", "quota arithmetic overflow"))
    };
    Ok(c::QuotaAmounts {
        money_microusd: sum(a.money_microusd, b.money_microusd)?,
        requests: sum(a.requests, b.requests)?,
        tokens: sum(a.tokens, b.tokens)?,
        repairs: sum(a.repairs, b.repairs)?,
    })
}
fn reservation(usage: &c::Usage, repair: u64) -> Result<c::QuotaAmounts> {
    add(
        &zero(),
        &c::QuotaAmounts {
            money_microusd: usage.money_microusd,
            requests: usage.requests,
            tokens: usage
                .input_tokens
                .checked_add(usage.output_tokens)
                .ok_or_else(|| invalid("token overflow"))?,
            repairs: repair,
        },
    )
}
fn fits(used: &c::QuotaAmounts, capacity: &c::QuotaAmounts, reset: u64) -> Result<()> {
    for (n, max, code) in [
        (
            used.money_microusd,
            capacity.money_microusd,
            "budget_exhausted",
        ),
        (used.requests, capacity.requests, "request_quota"),
        (used.tokens, capacity.tokens, "token_quota"),
        (used.repairs, capacity.repairs, "repair_exhausted"),
    ] {
        if n > max {
            return fail(code, &format!("retry_at={reset}"));
        }
    }
    Ok(())
}
fn matches(scope: &c::ProviderQuotaScope, ctx: &c::ProviderContext, model: &str) -> bool {
    match scope {
        c::ProviderQuotaScope::Provider { provider } => provider == &ctx.provider,
        c::ProviderQuotaScope::Project {
            provider,
            project_id,
        } => provider == &ctx.provider && project_id == &ctx.project_id,
        c::ProviderQuotaScope::Model {
            provider,
            project_id,
            model: m,
        } => provider == &ctx.provider && project_id == &ctx.project_id && m == model,
    }
}
fn project_key(ctx: &c::ProviderContext) -> String {
    c::ProviderQuotaScope::Project {
        provider: ctx.provider.clone(),
        project_id: ctx.project_id.clone(),
    }
    .key()
}
impl Broker {
    pub(super) fn new(store: Store) -> Self {
        Self {
            store,
            clock: Arc::new(SystemClock),
            #[cfg(test)]
            synthetic: false,
        }
    }
    #[cfg(test)]
    pub(super) fn synthetic(store: Store, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock,
            synthetic: true,
        }
    }
    fn enabled(&self) -> bool {
        #[cfg(test)]
        {
            self.synthetic
        }
        #[cfg(not(test))]
        {
            false
        }
    }
    fn gate(&self, tx: &Connection, run: &str, epoch: i64) -> Result<c::RunRequest> {
        Store::active(tx, run, epoch)?;
        let raw: String =
            tx.query_row("SELECT request FROM runs WHERE id=?1", [run], |r| r.get(0))?;
        let request: c::RunRequest = serde_json::from_str(&raw)?;
        if request.mode != c::ExecutionMode::Live || !self.enabled() {
            return fail(
                "live_disabled",
                "private adapter has no product dispatch authorization",
            );
        }
        let authorized:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM authorization) AND EXISTS(SELECT 1 FROM accounting_imports)",[],|r|r.get(0))?;
        if !authorized {
            return fail(
                "accounting_pending",
                "accounting and separate authorization are required",
            );
        }
        Ok(request)
    }
    pub(super) fn register(&self, b: &Binding) -> Result<()> {
        for s in [
            &b.id,
            &b.context.provider,
            &b.context.project_id,
            &b.context.credential_context_id,
            &b.model,
        ] {
            id(s)?;
        }
        if !valid_hash(&b.recipe_sha256) {
            return Err(invalid("recipe digest required"));
        }
        if b.recovery_estimate.requests == 0 {
            return Err(invalid(
                "a status RPC must reserve request capacity even when money is zero",
            ));
        }
        reservation(&b.recovery_estimate, 0)?;
        let encoded = serde_json::to_string(b)?;
        self.store.transaction(|tx| {
            if let Some(old) = tx
                .query_row(
                    "SELECT record FROM provider_v2_bindings WHERE id=?1",
                    [&b.id],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                if old != encoded {
                    return fail("registry_conflict", "binding is immutable");
                }
                return Ok(());
            }
            tx.execute(
                "INSERT INTO provider_v2_bindings VALUES(?1,?2)",
                params![b.id, encoded],
            )?;
            Ok(())
        })
    }
    pub(super) fn authorize_unit(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        binding: &str,
        operation_id: &str,
        repair: Option<c::CreativeRepairIdentity>,
    ) -> Result<()> {
        self.authorize_grant(run, epoch, unit, binding, operation_id, repair, None)
    }
    pub(super) fn authorize_retry_unit(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        binding: &str,
        operation_id: &str,
        repair: Option<c::CreativeRepairIdentity>,
        previous: &str,
    ) -> Result<()> {
        id(previous)?;
        self.authorize_grant(
            run,
            epoch,
            unit,
            binding,
            operation_id,
            repair,
            Some(previous.into()),
        )
    }
    fn authorize_grant(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        binding: &str,
        operation_id: &str,
        repair: Option<c::CreativeRepairIdentity>,
        retry: Option<String>,
    ) -> Result<()> {
        id(unit)?;
        id(operation_id)?;
        let record = serde_json::to_string(&Unit {
            binding: binding.into(),
            operation_id: operation_id.into(),
            repair,
            retry,
        })?;
        self.store.transaction(|tx| {
            Store::active(tx, run, epoch)?;
            let _: String = tx.query_row(
                "SELECT record FROM provider_v2_bindings WHERE id=?1",
                [binding],
                |r| r.get(0),
            )?;
            if let Some(old) = tx
                .query_row(
                    "SELECT record FROM provider_v2_units WHERE run=?1 AND epoch=?2 AND unit=?3",
                    params![run, epoch, unit],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                if old != record {
                    return fail("unit_conflict", "unit authority is immutable");
                }
                return Ok(());
            }
            tx.execute(
                "INSERT INTO provider_v2_units VALUES(?1,?2,?3,?4)",
                params![run, epoch, unit, record],
            )?;
            Ok(())
        })
    }
    pub(super) fn configure_window(&self, w: &Window) -> Result<()> {
        id(&w.id)?;
        add(&zero(), &w.capacity)?;
        if w.start_at_ms >= w.reset_at_ms || w.reset_at_ms > c::MAX_SAFE_INTEGER {
            return fail("invalid_window", "invalid quota interval");
        }
        self.store.transaction(|tx|{if let Some(old)=tx.query_row("SELECT record FROM provider_v2_windows WHERE id=?1",[&w.id],|r|r.get::<_,String>(0)).optional()?{let old:Window=serde_json::from_str(&old)?;if old.scope!=w.scope||old.start_at_ms!=w.start_at_ms||old.reset_at_ms!=w.reset_at_ms{return fail("window_conflict","quota window identity is immutable");}}let records=tx.prepare("SELECT record FROM provider_v2_windows WHERE id<>?1")?.query_map([&w.id],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;for raw in records{let old:Window=serde_json::from_str(&raw)?;if old.scope==w.scope&&old.start_at_ms==w.start_at_ms&&old.reset_at_ms==w.reset_at_ms{return fail("window_conflict","scope/interval has another ID");}}tx.execute("INSERT INTO provider_v2_windows(id,scope_key,record) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET record=excluded.record",params![w.id,w.scope.key(),serde_json::to_string(w)?])?;Ok(())})
    }
    fn window_snapshot(tx: &Connection, w: &Window) -> Result<c::ProviderQuotaWindow> {
        let mut used = zero();
        let records=tx.prepare("SELECT context,model,accounted FROM provider_v2_attempts WHERE created>=?1 AND created<?2 UNION ALL SELECT context,model,accounted FROM provider_v2_queries WHERE created>=?1 AND created<?2")?.query_map(params![w.start_at_ms,w.reset_at_ms],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        for (ctx, model, raw) in records {
            let ctx: c::ProviderContext = serde_json::from_str(&ctx)?;
            if matches(&w.scope, &ctx, &model) {
                used = add(&used, &serde_json::from_str(&raw)?)?;
            }
        }
        let (inherited, inherited_retry) = legacy::snapshot(tx, w)?;
        used = add(&used, &inherited)?;
        let retry: u64 = tx.query_row(
            "SELECT retry_at FROM provider_v2_windows WHERE id=?1",
            [&w.id],
            |r| r.get(0),
        )?;
        Ok(c::ProviderQuotaWindow {
            window_id: w.id.clone(),
            scope: w.scope.clone(),
            start_at_ms: w.start_at_ms,
            reset_at_ms: w.reset_at_ms,
            retry_at_ms: if retry.max(inherited_retry) == 0 {
                None
            } else {
                Some(retry.max(inherited_retry))
            },
            capacity: w.capacity.clone(),
            accounted: used,
        })
    }
    pub(super) fn quota_snapshot(&self, id: &str) -> Result<c::ProviderQuotaWindow> {
        self.store.transaction(|tx| {
            legacy::sync(tx)?;
            self.quota_for_id(tx, id)
        })
    }
    pub(super) fn bind_legacy_scope(
        &self,
        legacy: &str,
        scope: &c::ProviderQuotaScope,
    ) -> Result<()> {
        self.store.transaction(|tx| legacy::bind(tx, legacy, scope))
    }
    fn windows(&self, tx: &Connection, b: &Binding) -> Result<Vec<c::ProviderQuotaWindow>> {
        let t = self.clock.now_ms();
        let records = tx
            .prepare("SELECT record FROM provider_v2_windows ORDER BY id")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut windows = vec![];
        for raw in records {
            let w: Window = serde_json::from_str(&raw)?;
            if matches(&w.scope, &b.context, &b.model) && w.start_at_ms <= t && t < w.reset_at_ms {
                windows.push(Self::window_snapshot(tx, &w)?);
            }
        }
        if windows.is_empty()
            || windows.len() > 16
            || !windows
                .iter()
                .any(|w| matches!(w.scope, c::ProviderQuotaScope::Project { .. }))
        {
            return fail(
                "limits_unknown",
                "all applicable quotas including a project window are required",
            );
        }
        if let Some(retry) = windows
            .iter()
            .filter_map(|w| w.retry_at_ms)
            .max()
            .filter(|r| t < *r)
        {
            return fail("provider_wait", &format!("retry_at={retry}"));
        }
        Ok(windows)
    }
    fn binding(tx: &Connection, run: &str, epoch: i64, unit: &str) -> Result<(Unit, Binding)> {
        let raw: String = tx
            .query_row(
                "SELECT record FROM provider_v2_units WHERE run=?1 AND epoch=?2 AND unit=?3",
                params![run, epoch, unit],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| invalid("unit lacks runtime authority"))?;
        let unit: Unit = serde_json::from_str(&raw)?;
        let raw: String = tx.query_row(
            "SELECT record FROM provider_v2_bindings WHERE id=?1",
            [&unit.binding],
            |r| r.get(0),
        )?;
        Ok((unit, serde_json::from_str(&raw)?))
    }
    fn verify_artifact(&self, tx: &Connection, run: &str, a: &c::ArtifactRef) -> Result<()> {
        let encoded: Option<String> = tx
            .query_row(
                "SELECT reference FROM artifacts WHERE id=?1 AND run=?2",
                params![a.id, run],
                |r| r.get(0),
            )
            .optional()?;
        if encoded
            .map(|s| serde_json::from_str::<serde_json::Value>(&s))
            .transpose()?
            != Some(serde_json::to_value(a)?)
        {
            return Err(invalid(
                "artifact outside the issued run or substituted metadata",
            ));
        }
        let verified = self.store.open_artifact(&a.id)?;
        if verified.sha256 != a.file.sha256
            || verified.bytes != a.file.bytes
            || verified.media_type != a.file.media_type
        {
            return Err(invalid("artifact metadata changed"));
        }
        Ok(())
    }
    pub(super) fn verify_service_content(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        p: &c::ProviderOperation,
    ) -> Result<()> {
        self.store.transaction(|tx| {
            let request = self.gate(tx, run, epoch)?;
            legacy::sync(tx)?;
            self.check_budgets(tx, &request, run, &zero())?;
            let (_, b) = Self::binding(tx, run, epoch, unit)?;
            if p.unit_id != unit
                || p.provider != b.context.provider
                || p.model != b.model
                || p.recipe_sha256 != b.recipe_sha256
                || p.kind != b.kind
            {
                return Err(invalid("content differs from runtime unit authority"));
            }
            self.verify_artifact(tx, run, &p.payload)?;
            for a in &p.inputs {
                self.verify_artifact(tx, run, a)?;
            }
            Ok(())
        })
    }
    pub(super) fn verify_result_bytes(&self, result: &c::ProviderResultV2) -> Result<()> {
        if let Some(raw) = &result.result.raw_response {
            if raw.file.bytes > 1024 * 1024 {
                return fail("receipt_bounds", "private receipt exceeds its envelope");
            }
            raw.file
                .verify(
                    &self
                        .store
                        .read_object(&raw.file.sha256, raw.file.bytes as usize)?,
                )
                .map_err(contract)?;
        }
        self.store.transaction(|tx| {
            for a in &result.result.outputs {
                self.verify_artifact(tx, &result.identity.run_id, a)?;
            }
            Ok(())
        })
    }
    fn verify_issued_binding(tx: &Connection, op: &c::ProviderOperationV2) -> Result<()> {
        let (unit, b) = Self::binding(
            tx,
            &op.identity.run_id,
            op.identity.epoch as i64,
            &op.operation.unit_id,
        )?;
        if unit.operation_id != op.identity.operation_id
            || unit.repair != op.identity.repair
            || b.context != op.context
            || b.model != op.operation.model
            || b.kind != op.operation.kind
            || b.recipe_sha256 != op.operation.recipe_sha256
        {
            return Err(invalid("issuance differs from the current unit grant"));
        }
        Ok(())
    }
    fn total(tx: &Connection, run: Option<&str>) -> Result<c::QuotaAmounts> {
        let mut total = zero();
        let records = tx
            .prepare("SELECT accounted FROM provider_v2_attempts WHERE (?1 IS NULL OR run=?1) UNION ALL SELECT accounted FROM provider_v2_queries WHERE (?1 IS NULL OR run=?1)")?
            .query_map([run], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for raw in records {
            total = add(&total, &serde_json::from_str(&raw)?)?;
        }
        let values:(u64,u64,u64,u64)=tx.query_row("SELECT COALESCE(SUM(money),0),COALESCE(SUM(requests),0),COALESCE(SUM(tokens),0),COALESCE(SUM(repair),0) FROM attempts WHERE (?1 IS NULL OR run=?1)",[run],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        add(
            &total,
            &c::QuotaAmounts {
                money_microusd: values.0,
                requests: values.1,
                tokens: values.2,
                repairs: values.3,
            },
        )
    }
    fn check_budgets(
        &self,
        tx: &Connection,
        request: &c::RunRequest,
        run: &str,
        usage: &c::QuotaAmounts,
    ) -> Result<()> {
        fits(
            &add(&Self::total(tx, Some(run))?, usage)?,
            &c::QuotaAmounts {
                money_microusd: request.limits.money_microusd,
                requests: request.limits.requests,
                tokens: request.limits.tokens,
                repairs: request.limits.repairs as u64,
            },
            0,
        )?;
        let mut total = Self::total(tx, None)?.money_microusd;
        let imports = tx
            .prepare("SELECT record FROM accounting_imports")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for raw in imports {
            let import: super::broker::AccountingImport = serde_json::from_str(&raw)?;
            total = total
                .checked_add(
                    u64::try_from(import.money)
                        .map_err(|_| invalid("negative inherited accounting"))?,
                )
                .ok_or_else(|| invalid("accounting overflow"))?;
        }
        let cap: u64 =
            tx.query_row("SELECT cap FROM authorization WHERE id=1", [], |r| r.get(0))?;
        if total
            .checked_add(usage.money_microusd)
            .is_none_or(|n| n > cap)
        {
            return fail(
                "budget_exhausted",
                "lifetime product authorization is exhausted",
            );
        }
        Ok(())
    }
    fn unresolved(
        &self,
        tx: &Connection,
        b: &Binding,
        proposal: &c::ProviderOperation,
    ) -> Result<()> {
        let records=tx.prepare("SELECT descriptor FROM provider_v2_attempts WHERE state IN ('reserved','dispatched','uncertain')")?.query_map([],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
        for raw in records {
            let old: c::ProviderOperationV2 = serde_json::from_str(&raw)?;
            if old.context.provider == b.context.provider
                && old.context.project_id == b.context.project_id
                && old.operation.model == b.model
                && old.operation.request_sha256 == proposal.request_sha256
            {
                return fail(
                    "attempt_unresolved",
                    "matching work remains held across runs and credential contexts",
                );
            }
        }
        Ok(())
    }
    pub(super) fn bind_provider_proposal(
        &self,
        run: &str,
        epoch: i64,
        runtime_unit: &str,
        proposal: &c::ProviderOperation,
    ) -> Result<c::ProviderOperationV2> {
        self.bind(run, epoch, runtime_unit, proposal, None, false)
    }
    pub(super) fn retry_provider_proposal(
        &self,
        run: &str,
        epoch: i64,
        runtime_unit: &str,
        previous: &str,
        proposal: &c::ProviderOperation,
    ) -> Result<c::ProviderOperationV2> {
        self.bind(run, epoch, runtime_unit, proposal, Some(previous), false)
    }
    pub(super) fn bind_service_proposal(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        p: &c::ProviderOperation,
    ) -> Result<c::ProviderOperationV2> {
        self.bind(run, epoch, unit, p, None, true)
    }
    fn semantic(proposal: &c::ProviderOperation) -> Result<String> {
        let mut value = serde_json::to_value(proposal)?;
        value["attempt_id"] = json!("");
        value["payload"]["id"] = json!("");
        if let Some(inputs) = value["inputs"].as_array_mut() {
            for a in inputs {
                a["id"] = json!("");
            }
        }
        Ok(digest(&serde_json::to_vec(&value)?))
    }
    fn bind(
        &self,
        run: &str,
        epoch: i64,
        runtime_unit: &str,
        proposal: &c::ProviderOperation,
        previous: Option<&str>,
        from_grant: bool,
    ) -> Result<c::ProviderOperationV2> {
        let result=self.store.transaction(|tx|{
            let request=self.gate(tx,run,epoch)?;if proposal.unit_id!=runtime_unit{return Err(invalid("caller unit differs from the active runtime unit"));}legacy::sync(tx)?;
            let(unit,b)=Self::binding(tx,run,epoch,runtime_unit)?;
            let previous=if from_grant {unit.retry.as_deref()}else{previous};
            if b.context.provider!=proposal.provider||b.model!=proposal.model||b.recipe_sha256!=proposal.recipe_sha256||b.kind!=proposal.kind{return Err(invalid("proposal differs from runtime binding"));}
            let key=digest(format!("{}:{}",project_key(&b.context),unit.operation_id).as_bytes());
            let predecessor=if let Some(previous)=previous{let row:Option<(String,String,u64,String)>=tx.query_row("SELECT descriptor,state,retry_index,operation_key FROM provider_v2_attempts WHERE id=?1",[previous],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;Some(row.ok_or_else(||invalid("retry predecessor is unknown"))?)}else{None};
            let index=predecessor.as_ref().map(|p|p.2.checked_add(1).ok_or_else(||invalid("retry ordinal overflow"))).transpose()?.unwrap_or(0);
            let attempt=digest(format!("{run}:v2:{key}:{index}").as_bytes());
            let mut operation=proposal.clone();operation.attempt_id=String::new();let proposal_hash=digest(&serde_json::to_vec(&operation)?);
            if let Some((old,raw))=tx.query_row("SELECT proposal_hash,descriptor FROM provider_v2_attempts WHERE id=?1",[&attempt],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?{if old!=proposal_hash{return fail("operation_conflict","issued attempt differs");}return Ok(serde_json::from_str(&raw)?);}
            let semantic=Self::semantic(proposal)?;
            let logical=serde_json::to_string(&json!({"fingerprint":semantic,"context":b.context,"model":b.model,"kind":b.kind,"recipe_sha256":b.recipe_sha256,"repair":unit.repair}))?;
            let old:Option<String>=tx.query_row("SELECT record FROM provider_v2_operations WHERE id=?1",[&key],|r|r.get(0)).optional()?;
            let retry=if let Some((raw,state,old_index,previous_key))=&predecessor{
                let old_op:c::ProviderOperationV2=serde_json::from_str(raw)?;
                if previous_key!=&key||old.as_deref()!=Some(&logical)||old_op.context!=b.context||old_op.identity.repair!=unit.repair{return fail("operation_conflict","retry changed logical operation, context, repair, request or recipe");}
                if state!="rejected"{return fail("attempt_unresolved","only a known rejection permits an original-send retry");}
                let newer:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM provider_v2_attempts WHERE operation_key=?1 AND retry_index>?2)",params![key,old_index],|r|r.get(0))?;
                if newer{return fail("retry_lineage","retry must extend the latest attempt");}
                Some(c::TransportRetryIdentity{previous_attempt_id:old_op.operation.attempt_id,previous_request_sha256:old_op.operation.request_sha256,retry_index:index})
            }else{if let Some(old)=&old{let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM provider_v2_attempts WHERE operation_key=?1 AND state IN ('reserved','dispatched','uncertain'))",[&key],|r|r.get(0))?;if pending{return fail("attempt_unresolved","logical operation has unresolved work");}if old!=&logical{return fail("operation_conflict","logical descriptor is immutable");}return fail("retry_lineage","an existing logical operation requires an explicit predecessor");}None};
            let mut repair_charge=0;
            if let Some(repair)=&unit.repair{
                let repair_key=digest(format!("{}:{}",project_key(&b.context),repair.repair_id).as_bytes());
                let existing:Option<String>=tx.query_row("SELECT operation_key FROM provider_v2_repairs WHERE id=?1",[&repair_key],|r|r.get(0)).optional()?;
                if let Some(existing)=existing{if existing!=key{return fail("repair_conflict","repair ID already belongs to another logical operation");}}
                else{
                    if retry.is_some(){return fail("repair_conflict","retry cannot create a new creative repair charge");}
                    let previous_key=digest(format!("{}:{}",project_key(&b.context),repair.previous_operation_id).as_bytes());
                    let row:Option<(String,String)>=tx.query_row("SELECT descriptor,state FROM provider_v2_attempts WHERE operation_key=?1 ORDER BY retry_index DESC LIMIT 1",[&previous_key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
                    let(raw,state)=row.ok_or_else(||invalid("creative repair predecessor is unknown"))?;let previous:c::ProviderOperationV2=serde_json::from_str(&raw)?;
                    if !matches!(state.as_str(),"completed"|"rejected"){return fail("attempt_unresolved","creative repair cannot repurchase unresolved predecessor work");}
                    if previous.context!=b.context||previous.operation.model!=b.model||previous.operation.kind!=b.kind||previous.operation.recipe_sha256!=b.recipe_sha256||repair.repair_index!=previous.identity.repair.as_ref().map_or(1,|p|p.repair_index+1){return fail("repair_lineage","creative repair identity/ordinal differs from its predecessor");}
                    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM provider_v2_repairs WHERE previous_key=?1 AND ordinal=?2)",params![previous_key,repair.repair_index],|r|r.get(0))?;if used{return fail("repair_lineage","creative repair branch already exists");}
                    tx.execute("INSERT INTO provider_v2_repairs VALUES(?1,?2,?3,?4)",params![repair_key,key,previous_key,repair.repair_index])?;repair_charge=1;
                }
            }
            self.unresolved(tx,&b,proposal)?;
            self.verify_artifact(tx,run,&proposal.payload)?;if proposal.payload.file.bytes>1024*1024{return Err(invalid("payload exceeds 1 MiB"));}for a in &proposal.inputs{self.verify_artifact(tx,run,a)?;}
            let usage=reservation(&proposal.estimate,repair_charge)?;self.check_budgets(tx,&request,run,&usage)?;
            let mut windows=self.windows(tx,&b)?;for w in &mut windows{w.accounted=add(&w.accounted,&usage)?;fits(&w.accounted,&w.capacity,w.reset_at_ms)?;}
            operation.attempt_id=attempt.clone();let issued=c::ProviderOperationV2{schema_version:2,operation,context:b.context.clone(),identity:c::ProviderAttemptIdentity{run_id:run.into(),epoch:epoch as u64,operation_id:unit.operation_id.clone(),repair:unit.repair,transport_retry:retry},quota_windows:windows,recovery:None};issued.validate().map_err(contract)?;
            tx.execute("INSERT OR IGNORE INTO provider_v2_operations VALUES(?1,?2,?3)",params![key,unit.operation_id,logical])?;
            tx.execute("INSERT INTO provider_v2_attempts(id,operation_key,retry_index,run,epoch,context,model,proposal_hash,descriptor,state,reserved,accounted,created) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'reserved',?10,?10,?11)",params![attempt,key,index,run,epoch,serde_json::to_string(&b.context)?,b.model,proposal_hash,serde_json::to_string(&issued)?,serde_json::to_string(&usage)?,self.clock.now_ms()])?;
            Store::emit(tx,run,"budget_reserved",&json!({"attempt_id":attempt,"schema_version":2,"window_ids":issued.quota_windows.iter().map(|w|&w.window_id).collect::<Vec<_>>(),"accounted":usage,"repair_charged":repair_charge,"synthetic":true}))?;Ok(issued)
        });
        if let Err(error) = &result {
            self.park_error(run, epoch, runtime_unit, Some(&proposal.estimate), error)?;
        }
        result
    }
}

#[derive(Clone)]
pub(super) struct Job {
    pub id: String,
    pub status: c::ProviderJobStatus,
}
#[derive(Clone)]
pub(super) struct Retry {
    pub reason: c::ProviderRetryReason,
    pub not_before_ms: u64,
    pub retry_after_ms: Option<u64>,
    pub window_ids: Vec<String>,
}
#[derive(Clone)]
pub(super) struct Reply {
    pub attempt_id: String,
    pub request_sha256: String,
    pub completion: c::ProviderCompletion,
    pub usage: c::ObservedProviderUsage,
    pub raw: Option<Vec<u8>>,
    pub outputs: Vec<c::ArtifactRef>,
    pub job: Option<Job>,
    pub retry: Option<Retry>,
}
impl Reply {
    pub(super) fn ambiguous(op: &c::ProviderOperationV2) -> Self {
        Self {
            attempt_id: op.operation.attempt_id.clone(),
            request_sha256: op.operation.request_sha256.clone(),
            completion: c::ProviderCompletion::Ambiguous,
            usage: c::ObservedProviderUsage {
                requests: None,
                tokens: None,
                input_tokens: None,
                output_tokens: None,
                money_microusd: None,
            },
            raw: None,
            outputs: vec![],
            job: None,
            retry: None,
        }
    }
}
#[async_trait::async_trait]
pub(super) trait OfflineTransport: Send + Sync {
    async fn send(&self, operation: &c::ProviderOperationV2, exact_payload: &[u8])
    -> Result<Reply>;
    async fn recover(&self, _operation: &c::ProviderOperationV2) -> Result<RecoveryReply> {
        fail(
            "recovery_unavailable",
            "fake recovery transport is not installed",
        )
    }
}
#[derive(Clone)]
pub(super) struct RecoveryReply {
    pub original: Reply,
    pub query_usage: c::ObservedProviderUsage,
}
fn retained(
    reserved: &c::QuotaAmounts,
    usage: &c::ObservedProviderUsage,
    ambiguous: bool,
) -> Result<c::QuotaAmounts> {
    let metric = |known: Option<u64>, reserve: u64| {
        if ambiguous {
            reserve.max(known.unwrap_or(reserve))
        } else {
            known.unwrap_or(reserve)
        }
    };
    let tokens = usage.tokens.or_else(|| {
        usage
            .input_tokens
            .zip(usage.output_tokens)
            .and_then(|(a, b)| a.checked_add(b))
    });
    add(
        &zero(),
        &c::QuotaAmounts {
            money_microusd: metric(usage.money_microusd, reserved.money_microusd),
            requests: metric(usage.requests, reserved.requests),
            tokens: metric(tokens, reserved.tokens).max(
                usage
                    .input_tokens
                    .unwrap_or(0)
                    .checked_add(usage.output_tokens.unwrap_or(0))
                    .ok_or_else(|| invalid("token overflow"))?,
            ),
            repairs: reserved.repairs,
        },
    )
}
impl Broker {
    fn park(tx: &Connection, run: &str, epoch: i64, deadline: u64) -> Result<()> {
        if Store::active(tx, run, epoch).is_err() {
            return Ok(());
        }
        tx.execute("UPDATE runs SET state='waiting_provider',retry_at=MAX(COALESCE(retry_at,0),?2) WHERE id=?1",params![run,deadline])?;
        Ok(())
    }
    fn retain_original_charges(
        reserved: &c::QuotaAmounts,
        prior: &c::QuotaAmounts,
        usage: &c::ObservedProviderUsage,
        ambiguous: bool,
        current: &mut c::QuotaAmounts,
    ) {
        let tokens = usage.tokens.or_else(|| {
            usage
                .input_tokens
                .zip(usage.output_tokens)
                .and_then(|(a, b)| a.checked_add(b))
        });
        for (charged, previous, estimate, observed) in [
            (
                &mut current.money_microusd,
                prior.money_microusd,
                reserved.money_microusd,
                usage.money_microusd,
            ),
            (
                &mut current.requests,
                prior.requests,
                reserved.requests,
                usage.requests,
            ),
            (&mut current.tokens, prior.tokens, reserved.tokens, tokens),
        ] {
            if ambiguous || observed.is_none() || previous > estimate {
                *charged = (*charged).max(previous);
            }
        }
        current.repairs = current.repairs.max(prior.repairs);
    }
    fn park_error(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        estimate: Option<&c::Usage>,
        error: &Error,
    ) -> Result<()> {
        if !matches!(
            error.code,
            "provider_wait"
                | "request_quota"
                | "token_quota"
                | "repair_exhausted"
                | "budget_exhausted"
        ) {
            return Ok(());
        }
        // Only persisted windows/observations supply deadlines. Diagnostic text has no authority.
        self.store.transaction(|tx| {
            if Store::active(tx, run, epoch).is_err() {
                return Ok(());
            }
            let (grant, b) = Self::binding(tx, run, epoch, unit)?;
            legacy::sync(tx)?;
            let repair = if estimate.is_some() {
                if let Some(repair) = &grant.repair {
                    let key = digest(
                        format!("{}:{}", project_key(&b.context), repair.repair_id).as_bytes(),
                    );
                    let used: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM provider_v2_repairs WHERE id=?1)",
                        [key],
                        |r| r.get(0),
                    )?;
                    u64::from(!used)
                } else {
                    0
                }
            } else {
                0
            };
            let cost = reservation(estimate.unwrap_or(&b.recovery_estimate), repair)?;
            let request: c::RunRequest = serde_json::from_str(&tx.query_row(
                "SELECT request FROM runs WHERE id=?1",
                [run],
                |r| r.get::<_, String>(0),
            )?)?;
            if self.check_budgets(tx, &request, run, &cost).is_err() {
                return Ok(());
            }
            let time = self.clock.now_ms();
            let records = tx
                .prepare("SELECT record FROM provider_v2_windows")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut deadline = 0;
            for raw in records {
                let w: Window = serde_json::from_str(&raw)?;
                if !matches(&w.scope, &b.context, &b.model)
                    || time < w.start_at_ms
                    || time >= w.reset_at_ms
                {
                    continue;
                }
                let current = Self::window_snapshot(tx, &w)?;
                if error.code == "provider_wait" {
                    deadline = deadline.max(current.retry_at_ms.unwrap_or(0));
                    continue;
                }
                let used = add(&current.accounted, &cost)?;
                let exhausted = match error.code {
                    "request_quota" => used.requests > current.capacity.requests,
                    "token_quota" => used.tokens > current.capacity.tokens,
                    "repair_exhausted" => used.repairs > current.capacity.repairs,
                    "budget_exhausted" => used.money_microusd > current.capacity.money_microusd,
                    _ => false,
                };
                if exhausted {
                    deadline = deadline.max(current.reset_at_ms);
                }
            }
            if deadline <= time {
                return Ok(());
            }
            Self::park(tx, run, epoch, deadline)?;
            if Store::active(tx, run, epoch).is_ok() {
                Store::emit(
                    tx,
                    run,
                    "provider_wait",
                    &json!({"source":"provider-v2","reason":error.code,"retry_at":deadline}),
                )?;
            }
            Ok(())
        })
    }
    fn original(tx: &Connection, op: &c::ProviderOperationV2) -> Result<(String, Option<String>)> {
        op.validate().map_err(contract)?;
        let row: Option<(String, String, Option<String>)> = tx
            .query_row(
                "SELECT descriptor,state,result FROM provider_v2_attempts WHERE id=?1",
                [&op.operation.attempt_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((stored, state, result)) = row else {
            return fail(
                "issued_descriptor_mismatch",
                "attempt was not issued by this runtime",
            );
        };
        if stored != serde_json::to_string(op)? {
            return fail(
                "issued_descriptor_mismatch",
                "descriptor differs from immutable issuance",
            );
        }
        Ok((state, result))
    }
    pub(super) async fn execute_issued_provider(
        &self,
        op: &c::ProviderOperationV2,
        transport: &dyn OfflineTransport,
    ) -> Result<c::ProviderResultV2> {
        let(existing,bytes)=self.store.transaction(|tx|{let request=self.gate(tx,&op.identity.run_id,op.identity.epoch as i64)?;legacy::sync(tx)?;self.check_budgets(tx,&request,&op.identity.run_id,&zero())?;let(state,result)=Self::original(tx,op)?;Self::verify_issued_binding(tx,op)?;if matches!(state.as_str(),"completed"|"rejected"){return Ok((result,None));}if state!="reserved"{return fail("attempt_unresolved","original submission cannot be repeated");}let t=self.clock.now_ms();for w in &op.quota_windows{if t<w.start_at_ms||t>=w.reset_at_ms{return fail("limits_expired","issued quota interval expired; reservation remains held");}let current=self.quota_for_id(tx,&w.window_id)?;if current.retry_at_ms.is_some_and(|n|t<n){return fail("provider_wait",&format!("retry_at={}",current.retry_at_ms.unwrap()));}fits(&current.accounted,&current.capacity,current.reset_at_ms)?;}let(_,binding)=Self::binding(tx,&op.identity.run_id,op.identity.epoch as i64,&op.operation.unit_id)?;let applicable=self.windows(tx,&binding)?;if applicable.len()!=op.quota_windows.len()||applicable.iter().any(|w|!op.quota_windows.iter().any(|old|old.window_id==w.window_id)){return fail("limits_changed","applicable quota set changed after issuance");}tx.execute("UPDATE runs SET state='running',retry_at=NULL WHERE id=?1 AND epoch=?2 AND state='waiting_provider' AND retry_at<=?3",params![op.identity.run_id,op.identity.epoch,t])?;self.verify_artifact(tx,&op.identity.run_id,&op.operation.payload)?;for a in &op.operation.inputs{self.verify_artifact(tx,&op.identity.run_id,a)?;}let bytes=self.store.artifact_bounded(&op.operation.payload.id,1024*1024)?.0;op.operation.payload.file.verify(&bytes).map_err(contract)?;let t=self.clock.now_ms();if op.quota_windows.iter().any(|w|t<w.start_at_ms||t>=w.reset_at_ms){return fail("limits_expired","quota interval expired during preparation");}tx.execute("UPDATE provider_v2_attempts SET state='dispatched',started=?2 WHERE id=?1 AND state='reserved'",params![op.operation.attempt_id,t])?;Store::emit(tx,&op.identity.run_id,"provider_dispatched",&json!({"attempt_id":op.operation.attempt_id,"started_at_ms":t,"synthetic":true}))?;Ok((None,Some(bytes)))})?;
        if let Some(result) = existing {
            return Ok(serde_json::from_str(&result)?);
        }
        let bytes = bytes.ok_or_else(|| invalid("prepared request bytes missing"))?;
        let start = std::time::Instant::now();
        let reply = match tokio::time::timeout(
            std::time::Duration::from_millis(op.operation.timeout_ms),
            transport.send(op, &bytes),
        )
        .await
        {
            Ok(Ok(reply)) => reply,
            _ => Reply::ambiguous(op),
        };
        self.record_reply(op, reply, start.elapsed().as_millis() as u64)
    }
    fn quota_for_id(&self, tx: &Connection, id: &str) -> Result<c::ProviderQuotaWindow> {
        let raw: String = tx.query_row(
            "SELECT record FROM provider_v2_windows WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        Self::window_snapshot(tx, &serde_json::from_str(&raw)?)
    }
    fn record_reply(
        &self,
        op: &c::ProviderOperationV2,
        reply: Reply,
        elapsed: u64,
    ) -> Result<c::ProviderResultV2> {
        self.persist_reply(op, reply, elapsed, None)
    }
    fn persist_reply(
        &self,
        op: &c::ProviderOperationV2,
        mut reply: Reply,
        elapsed: u64,
        query: Option<recovery::Settlement>,
    ) -> Result<c::ProviderResultV2> {
        let observed_at = query
            .as_ref()
            .map(|q| q.observed_at_ms)
            .unwrap_or_else(|| self.clock.now_ms());
        if let Some(job) = &reply.job {
            reply.completion = match job.status {
                c::ProviderJobStatus::Unknown
                | c::ProviderJobStatus::Submitted
                | c::ProviderJobStatus::Running => c::ProviderCompletion::Ambiguous,
                c::ProviderJobStatus::Succeeded => c::ProviderCompletion::Completed,
                c::ProviderJobStatus::Failed => c::ProviderCompletion::KnownRejection,
            };
        }
        if reply.attempt_id != op.operation.attempt_id
            || reply.request_sha256 != op.operation.request_sha256
        {
            return fail(
                "receipt_conflict",
                "transport response identifies another attempt/request",
            );
        }
        let raw = if let Some(bytes) = &reply.raw {
            if bytes.len() > 1024 * 1024 {
                return fail("receipt_bounds", "raw provider receipt exceeds 1 MiB");
            }
            let hash = self.store.put(bytes)?;
            Some(c::ArtifactRef {
                id: digest(format!("{}:{hash}", op.operation.attempt_id).as_bytes()),
                file: c::FileRef {
                    path: format!("provider/receipts/{hash}.bin"),
                    sha256: hash,
                    bytes: bytes.len() as u64,
                    media_type: "application/octet-stream".into(),
                    dimensions: None,
                },
                provenance: c::Provenance::Synthetic,
            })
        } else {
            None
        };
        self.store.transaction(|tx|{
   if let Some(query)=&query{self.settle_query(tx,query)?;}

   let(started,reserved,prior_job):(Option<u64>,String,Option<String>)=tx.query_row("SELECT started,reserved,job FROM provider_v2_attempts WHERE id=?1",[&op.operation.attempt_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;let started=started.ok_or_else(||invalid("no actual dispatch timestamp"))?;let reserved:c::QuotaAmounts=serde_json::from_str(&reserved)?;let mut accounted=retained(&reserved,&reply.usage,reply.completion==c::ProviderCompletion::Ambiguous)?;if query.is_some(){let previous:String=tx.query_row("SELECT accounted FROM provider_v2_attempts WHERE id=?1",[&op.operation.attempt_id],|r|r.get(0))?;Self::retain_original_charges(&reserved,&serde_json::from_str(&previous)?,&reply.usage,reply.completion==c::ProviderCompletion::Ambiguous,&mut accounted);}let t=self.clock.now_ms();
   let prior_job:Option<c::ExistingProviderJob>=prior_job.map(|s|serde_json::from_str(&s)).transpose()?;
   let recovery=if let Some(job)=&reply.job{if let Some(old)=&prior_job{if old.provider_job_id!=job.id||(matches!(old.status,c::ProviderJobStatus::Succeeded|c::ProviderJobStatus::Failed)&&old.status!=job.status){return fail("recovery_conflict","existing provider job identity or terminal status changed");}}Some(c::ExistingProviderJob{attempt_id:op.operation.attempt_id.clone(),request_sha256:op.operation.request_sha256.clone(),provider_job_id:job.id.clone(),status:job.status,observed_at_ms:observed_at})}else{prior_job};
   let state=if reply.completion==c::ProviderCompletion::Ambiguous||recovery.as_ref().is_some_and(|job|matches!(job.status,c::ProviderJobStatus::Unknown|c::ProviderJobStatus::Submitted|c::ProviderJobStatus::Running)){"uncertain"}else if reply.completion==c::ProviderCompletion::KnownRejection{"rejected"}else{"completed"};
   tx.execute("UPDATE provider_v2_attempts SET accounted=?2,state=?3,elapsed=?4,job=?5 WHERE id=?1",params![op.operation.attempt_id,serde_json::to_string(&accounted)?,state,elapsed,recovery.as_ref().map(serde_json::to_string).transpose()?])?;
   let mut retry=None;if let Some(observation)=&reply.retry{let mut deadline=observation.not_before_ms.max(observed_at);if let Some(delay)=observation.retry_after_ms{deadline=deadline.max(observed_at.checked_add(delay).ok_or_else(||invalid("retry overflow"))?);}for id in &observation.window_ids{if !op.quota_windows.iter().any(|w|&w.window_id==id){return Err(invalid("retry identifies an unrelated window"));}tx.execute("UPDATE provider_v2_windows SET retry_at=MAX(retry_at,?2) WHERE id=?1",params![id,deadline])?;}deadline=deadline.max(op.quota_windows.iter().filter_map(|w|self.quota_for_id(tx,&w.window_id).ok()?.retry_at_ms).max().unwrap_or(0));retry=Some(c::ProviderRetryTiming{reason:observation.reason,observed_at_ms:observed_at,not_before_ms:deadline,retry_after_ms:observation.retry_after_ms,window_ids:observation.window_ids.clone()});}
   if let Some(timing)=&retry{if timing.not_before_ms>t{Self::park(tx,&op.identity.run_id,op.identity.epoch as i64,timing.not_before_ms)?;}}
   if accounted.money_microusd>reserved.money_microusd||accounted.tokens>reserved.tokens||accounted.requests>reserved.requests{tx.execute("UPDATE runs SET state='failed',epoch=epoch+1,error='provider exceeded reservation' WHERE id=?1 AND epoch=?2 AND state IN ('running','waiting_provider')",params![op.identity.run_id,op.identity.epoch])?;}
   for a in &reply.outputs{self.verify_artifact(tx,&op.identity.run_id,a)?;}
   let mut windows=vec![];for old in &op.quota_windows{let mut current=self.quota_for_id(tx,&old.window_id)?;current.capacity=old.capacity.clone();windows.push(current);}
   let usage=reply.usage.requests.zip(reply.usage.input_tokens).zip(reply.usage.output_tokens).zip(reply.usage.money_microusd).map(|(((requests,input_tokens),output_tokens),money_microusd)|c::Usage{requests,input_tokens,output_tokens,money_microusd});
   let result=c::ProviderResultV2{schema_version:2,result:c::ProviderResult{attempt_id:op.operation.attempt_id.clone(),completion:reply.completion,request_sha256:op.operation.request_sha256.clone(),response_sha256:raw.as_ref().map(|a|a.file.sha256.clone()),raw_response:raw.clone(),outputs:reply.outputs,usage,started_at:started.to_string(),elapsed_ms:elapsed,diagnostic:"Offline injected transport observation; no product dispatch authorization.".into()},context:op.context.clone(),identity:op.identity.clone(),quota_windows:windows,observed_usage:reply.usage,accounted_usage:accounted,started_at_ms:started,recorded_at_ms:t,epoch_disposition:if Store::active(tx,&op.identity.run_id,op.identity.epoch as i64).is_ok(){c::ProviderEpochDisposition::Current}else{c::ProviderEpochDisposition::Obsolete},retry,recovery};result.validate_for_operation(op).map_err(contract)?;let encoded=serde_json::to_string(&result)?;let seq:u64=tx.query_row("SELECT COALESCE(MAX(seq),0)+1 FROM provider_v2_receipts WHERE attempt=?1",[&op.operation.attempt_id],|r|r.get(0))?;tx.execute("INSERT INTO provider_v2_receipts VALUES(?1,?2,?3,?4)",params![op.operation.attempt_id,seq,encoded,raw.as_ref().map(serde_json::to_string).transpose()?])?;tx.execute("UPDATE provider_v2_attempts SET result=?2 WHERE id=?1",params![op.operation.attempt_id,encoded])?;if let Some(query)=&query{tx.execute("UPDATE provider_v2_queries SET result=?2 WHERE id=?1",params![query.id,encoded])?;self.record_query_receipt(tx,query,&result)?;}Store::emit(tx,&op.identity.run_id,"provider_result",&json!({"attempt_id":op.operation.attempt_id,"schema_version":2,"accounted_usage":result.accounted_usage,"epoch_disposition":result.epoch_disposition,"receipt_seq":seq}))?;Ok(result)
  })
    }
    pub(super) fn raw_receipt(&self, attempt: &str, seq: u64) -> Result<Vec<u8>> {
        let raw: Option<String> = self.store.db()?.query_row(
            "SELECT raw FROM provider_v2_receipts WHERE attempt=?1 AND seq=?2",
            params![attempt, seq],
            |r| r.get(0),
        )?;
        let a: c::ArtifactRef = serde_json::from_str(
            &raw.ok_or_else(|| Error::new("not_found", "no raw response was observed"))?,
        )?;
        let bytes = self
            .store
            .read_object(&a.file.sha256, a.file.bytes as usize)?;
        a.file.verify(&bytes).map_err(contract)?;
        Ok(bytes)
    }
}
