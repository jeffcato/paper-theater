//! Status queries reserve independently and have no path to original submission.
use super::*;
pub(super) struct Settlement {
    pub id: String,
    pub usage: c::ObservedProviderUsage,
    pub ambiguous: bool,
    pub elapsed: u64,
    pub observed_at_ms: u64,
}
impl Broker {
    fn observer_grant(
        tx: &Connection,
        run: &str,
        epoch: i64,
        unit: &str,
        op: &c::ProviderOperationV2,
    ) -> Result<String> {
        let (authority, b) = Self::binding(tx, run, epoch, unit)?;
        if authority.repair.is_some()
            || b.context != op.context
            || b.model != op.operation.model
            || b.recipe_sha256 != op.operation.recipe_sha256
            || b.kind != op.operation.kind
        {
            return Err(invalid("observer lacks original work authority"));
        }
        let mut original = op.clone();
        original.recovery = None;
        Self::original(tx, &original)?;
        Ok(serde_json::to_string(
            &json!({"binding":authority.binding,"original_sha256":digest(&serde_json::to_vec(&original)?)}),
        )?)
    }
    pub(in crate::runtime) fn authorize_observer(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        attempt: &str,
    ) -> Result<()> {
        let op = self.recovery_descriptor(attempt)?;
        self.store.transaction(|tx| {
            self.gate(tx,run,epoch)?;let record=Self::observer_grant(tx,run,epoch,unit,&op)?;
            let old:Option<String>=tx.query_row("SELECT record FROM provider_v2_observers WHERE run=?1 AND epoch=?2 AND unit=?3 AND original_attempt=?4",params![run,epoch,unit,attempt],|r|r.get(0)).optional()?;
            if let Some(old)=old {if old!=record {return fail("unit_conflict","observer grant is immutable");}return Ok(());}
            tx.execute("INSERT INTO provider_v2_observers VALUES(?1,?2,?3,?4,?5)",params![run,epoch,unit,attempt,record])?;Ok(())
        })
    }
    pub(in crate::runtime) fn verify_observer(
        &self,
        run: &str,
        epoch: i64,
        unit: &str,
        op: &c::ProviderOperationV2,
    ) -> Result<()> {
        self.store.transaction(|tx| {
            let request=self.gate(tx,run,epoch)?;legacy::sync(tx)?;self.check_budgets(tx,&request,run,&zero())?;
            let expected=Self::observer_grant(tx,run,epoch,unit,op)?;
            let grant:Option<String>=tx.query_row("SELECT record FROM provider_v2_observers WHERE run=?1 AND epoch=?2 AND unit=?3 AND original_attempt=?4",params![run,epoch,unit,op.operation.attempt_id],|r|r.get(0)).optional()?;
            if grant.as_deref()!=Some(expected.as_str()) {return Err(invalid("separate observer grant required"));}
            self.verify_artifact(tx,&op.identity.run_id,&op.operation.payload)?;for a in &op.operation.inputs {self.verify_artifact(tx,&op.identity.run_id,a)?;}Ok(())
        })
    }
    pub(in crate::runtime) fn recovery_descriptor(
        &self,
        attempt: &str,
    ) -> Result<c::ProviderOperationV2> {
        let db = self.store.db()?;
        let (raw, job): (String, Option<String>) = db.query_row(
            "SELECT descriptor,job FROM provider_v2_attempts WHERE id=?1",
            [attempt],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut op: c::ProviderOperationV2 = serde_json::from_str(&raw)?;
        op.recovery = Some(serde_json::from_str(&job.ok_or_else(|| {
            Error::new("recovery_unknown", "no provider job was observed")
        })?)?);
        op.validate().map_err(contract)?;
        Ok(op)
    }
    pub(in crate::runtime) async fn recover_existing_provider(
        &self,
        run: &str,
        epoch: i64,
        unit_id: &str,
        op: &c::ProviderOperationV2,
        transport: &dyn OfflineTransport,
    ) -> Result<c::ProviderResultV2> {
        Ok(self
            .recover_inner(run, epoch, unit_id, op, transport, false)
            .await?
            .original)
    }
    pub(in crate::runtime) async fn recover_existing_receipt(
        &self,
        run: &str,
        epoch: i64,
        unit_id: &str,
        op: &c::ProviderOperationV2,
        transport: &dyn OfflineTransport,
    ) -> Result<c::ProviderRecoveryResultV2> {
        self.recover_inner(run, epoch, unit_id, op, transport, true)
            .await
    }
    async fn recover_inner(
        &self,
        run: &str,
        epoch: i64,
        unit_id: &str,
        op: &c::ProviderOperationV2,
        transport: &dyn OfflineTransport,
        require_observer: bool,
    ) -> Result<c::ProviderRecoveryResultV2> {
        let reservation=self.store.transaction(|tx|{
   let request=self.gate(tx,run,epoch)?;legacy::sync(tx)?;op.validate().map_err(contract)?;
   if require_observer {
       self.check_budgets(tx,&request,run,&zero())?;let expected=Self::observer_grant(tx,run,epoch,unit_id,op)?;let grant:Option<String>=tx.query_row("SELECT record FROM provider_v2_observers WHERE run=?1 AND epoch=?2 AND unit=?3 AND original_attempt=?4",params![run,epoch,unit_id,op.operation.attempt_id],|r|r.get(0)).optional()?;if grant.as_deref()!=Some(expected.as_str()) {return Err(invalid("separate observer grant required at query reservation"));}
       self.verify_artifact(tx,&op.identity.run_id,&op.operation.payload)?;for a in &op.operation.inputs {self.verify_artifact(tx,&op.identity.run_id,a)?;}
   }
   let(unit,b)=Self::binding(tx,run,epoch,unit_id)?;if unit.repair.is_some()||b.context!=op.context||b.model!=op.operation.model||b.recipe_sha256!=op.operation.recipe_sha256{return Err(invalid("recovery observer lacks the original context authority"));}
   let id=digest(format!("{}:{}:{}",project_key(&b.context),unit.operation_id,op.operation.attempt_id).as_bytes());let descriptor=digest(&serde_json::to_vec(&json!({"run":run,"epoch":epoch,"unit":unit_id,"original":op,"binding":b}))?);
   if let Some((old,state,result))=tx.query_row("SELECT descriptor_hash,state,result FROM provider_v2_queries WHERE id=?1",[&id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?))).optional()?{if old!=descriptor{return fail("operation_conflict","status-query identity already binds another descriptor");}if matches!(state.as_str(),"completed"|"rejected"){if result.is_none(){return fail("receipt_conflict","completed query result missing");}let receipt=self.saved_query(tx,&id,run,epoch,unit_id,op)?;return Ok((id,Some(serde_json::to_string(&receipt)?),0));}return fail("attempt_unresolved","an uncertain status query cannot be repurchased");}
   let mut original=op.clone();let job=original.recovery.take().ok_or_else(||Error::new("recovery_unknown","existing job identity required"))?;Self::original(tx,&original)?;
   let(stored_job,elapsed):(Option<String>,Option<u64>)=tx.query_row("SELECT job,elapsed FROM provider_v2_attempts WHERE id=?1",[&op.operation.attempt_id],|r|Ok((r.get(0)?,r.get(1)?)))?;let old:c::ExistingProviderJob=serde_json::from_str(&stored_job.ok_or_else(||invalid("existing job not recorded"))?)?;if old!=job{return fail("recovery_conflict","recovery must use the latest genuine existing-job observation");}
   let cost=reservation(&b.recovery_estimate,0)?;self.check_budgets(tx,&request,run,&cost)?;for w in self.windows(tx,&b)?{fits(&add(&w.accounted,&cost)?,&w.capacity,w.reset_at_ms)?;}
   let t=self.clock.now_ms();let encoded=serde_json::to_string(&cost)?;tx.execute("INSERT INTO provider_v2_queries(id,original_attempt,run,epoch,context,model,descriptor_hash,state,reserved,accounted,created,started) VALUES(?1,?2,?3,?4,?5,?6,?7,'dispatched',?8,?8,?9,?9)",params![id,op.operation.attempt_id,run,epoch,serde_json::to_string(&b.context)?,b.model,descriptor,encoded,t])?;
   let observer=c::ProviderObserverV2{schema_version:2,run_id:run.into(),epoch:epoch as u64,unit_id:unit_id.into()};let selector=c::ProviderRecoveryRequestV2{schema_version:2,original:op.clone()};tx.execute("INSERT INTO provider_v2_query_scopes VALUES(?1,?2,?3,NULL)",params![id,serde_json::to_string(&observer)?,serde_json::to_string(&selector)?])?;
   Store::emit(tx,run,"budget_reserved",&json!({"status_query":id,"original_attempt":op.operation.attempt_id,"accounted":cost,"started_at_ms":t,"synthetic":true}))?;Ok((id,None,elapsed.ok_or_else(||invalid("original dispatch duration is unknown"))?))
  });
        let (id, cached, original_elapsed) = match reservation {
            Ok(r) => r,
            Err(error) => {
                self.park_error(run, epoch, unit_id, None, &error)?;
                return Err(error);
            }
        };
        if let Some(result) = cached {
            return Ok(serde_json::from_str(&result)?);
        }
        let start = std::time::Instant::now();
        let (mut reply, usage, ambiguous) = match tokio::time::timeout(
            std::time::Duration::from_millis(op.operation.timeout_ms),
            transport.recover(op),
        )
        .await
        {
            Ok(Ok(reply)) => (reply.original, reply.query_usage, false),
            _ => (Reply::ambiguous(op), Reply::ambiguous(op).usage, true),
        };
        let observed_at_ms = self.clock.now_ms();
        if reply.job.is_none() {
            if let Some(job) = &op.recovery {
                reply.completion = match job.status {
                    c::ProviderJobStatus::Submitted
                    | c::ProviderJobStatus::Running
                    | c::ProviderJobStatus::Unknown => c::ProviderCompletion::Ambiguous,
                    c::ProviderJobStatus::Succeeded => c::ProviderCompletion::Completed,
                    c::ProviderJobStatus::Failed => c::ProviderCompletion::KnownRejection,
                };
            }
        }
        let query_id = id.clone();
        self.persist_reply(
            op,
            reply,
            original_elapsed,
            Some(Settlement {
                id,
                usage,
                ambiguous,
                elapsed: start.elapsed().as_millis() as u64,
                observed_at_ms,
            }),
        )?;
        let encoded: String = self.store.db()?.query_row(
            "SELECT receipt FROM provider_v2_query_scopes WHERE id=?1",
            [query_id],
            |r| r.get(0),
        )?;
        Ok(serde_json::from_str(&encoded)?)
    }
    fn saved_query(
        &self,
        tx: &Connection,
        id: &str,
        run: &str,
        epoch: i64,
        unit: &str,
        op: &c::ProviderOperationV2,
    ) -> Result<c::ProviderRecoveryResultV2> {
        let (observer, selector, receipt): (String, String, Option<String>) = tx
            .query_row(
                "SELECT observer,selector,receipt FROM provider_v2_query_scopes WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| invalid("legacy query lacks a complete observer receipt"))?;
        let expected = c::ProviderObserverV2 {
            schema_version: 2,
            run_id: run.into(),
            epoch: epoch as u64,
            unit_id: unit.into(),
        };
        let request = c::ProviderRecoveryRequestV2 {
            schema_version: 2,
            original: op.clone(),
        };
        if observer != serde_json::to_string(&expected)?
            || selector != serde_json::to_string(&request)?
        {
            return Err(invalid("cached query observer/selector differs"));
        }
        let receipt: c::ProviderRecoveryResultV2 = serde_json::from_str(
            &receipt.ok_or_else(|| invalid("completed query receipt missing"))?,
        )?;
        receipt
            .validate_for_request(&request, &expected)
            .map_err(contract)?;
        let (reserved,accounted,started,observed,result):(String,String,u64,String,String)=tx.query_row("SELECT reserved,accounted,started,observed,result FROM provider_v2_queries WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        let observed: serde_json::Value = serde_json::from_str(&observed)?;
        if receipt.query_id != id
            || receipt.query_reserved_usage != serde_json::from_str::<c::QuotaAmounts>(&reserved)?
            || receipt.query_accounted_usage != serde_json::from_str::<c::QuotaAmounts>(&accounted)?
            || receipt.query_started_at_ms != started
            || observed["recorded_at_ms"].as_u64() != Some(receipt.query_recorded_at_ms)
            || observed["usage"] != serde_json::to_value(&receipt.query_observed_usage)?
            || serde_json::from_str::<serde_json::Value>(&result)?
                != serde_json::to_value(&receipt.original)?
            || receipt.query_completion != c::ProviderCompletion::Completed
        {
            return fail(
                "receipt_conflict",
                "cached wrapper differs from durable query observations",
            );
        }
        let current: Option<String> = tx.query_row(
            "SELECT job FROM provider_v2_attempts WHERE id=?1",
            [&op.operation.attempt_id],
            |r| r.get(0),
        )?;
        if current
            .map(|s| serde_json::from_str::<c::ExistingProviderJob>(&s))
            .transpose()?
            != receipt.original.recovery
        {
            return fail(
                "recovery_conflict",
                "query cache no longer has the latest genuine job observation",
            );
        }
        Ok(receipt)
    }
    pub(super) fn record_query_receipt(
        &self,
        tx: &Connection,
        query: &Settlement,
        original: &c::ProviderResultV2,
    ) -> Result<()> {
        let (observer, selector): (String, String) = tx.query_row(
            "SELECT observer,selector FROM provider_v2_query_scopes WHERE id=?1",
            [&query.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let observer: c::ProviderObserverV2 = serde_json::from_str(&observer)?;
        let request: c::ProviderRecoveryRequestV2 = serde_json::from_str(&selector)?;
        let (reserved, accounted, started): (String, String, u64) = tx.query_row(
            "SELECT reserved,accounted,started FROM provider_v2_queries WHERE id=?1",
            [&query.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let receipt = c::ProviderRecoveryResultV2 {
            schema_version: 2,
            observer: observer.clone(),
            query_id: query.id.clone(),
            original: original.clone(),
            query_completion: if query.ambiguous {
                c::ProviderCompletion::Ambiguous
            } else {
                c::ProviderCompletion::Completed
            },
            query_reserved_usage: serde_json::from_str(&reserved)?,
            query_accounted_usage: serde_json::from_str(&accounted)?,
            query_observed_usage: query.usage.clone(),
            query_started_at_ms: started,
            query_recorded_at_ms: self.clock.now_ms(),
        };
        receipt
            .validate_for_request(&request, &observer)
            .map_err(contract)?;
        tx.execute("UPDATE provider_v2_queries SET observed=?2 WHERE id=?1",params![query.id,serde_json::to_string(&json!({"usage":query.usage,"observed_at_ms":query.observed_at_ms,"recorded_at_ms":receipt.query_recorded_at_ms}))?])?;
        tx.execute(
            "UPDATE provider_v2_query_scopes SET receipt=?2 WHERE id=?1",
            params![query.id, serde_json::to_string(&receipt)?],
        )?;
        Ok(())
    }
    pub(super) fn settle_query(&self, tx: &Connection, query: &Settlement) -> Result<()> {
        let (raw, run, epoch, state): (String, String, i64, String) = tx.query_row(
            "SELECT reserved,run,epoch,state FROM provider_v2_queries WHERE id=?1",
            [&query.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        if !matches!(state.as_str(), "dispatched" | "uncertain") {
            return fail("receipt_conflict", "status query already settled");
        }
        let reserved: c::QuotaAmounts = serde_json::from_str(&raw)?;
        let mut accounted = retained(&reserved, &query.usage, query.ambiguous)?;
        accounted.requests = accounted.requests.max(1);
        tx.execute("UPDATE provider_v2_queries SET accounted=?2,state=?3,elapsed=?4,observed=?5 WHERE id=?1",params![query.id,serde_json::to_string(&accounted)?,if query.ambiguous{"uncertain"}else{"completed"},query.elapsed,serde_json::to_string(&json!({"usage":query.usage,"observed_at_ms":query.observed_at_ms}))?])?;
        if accounted.money_microusd > reserved.money_microusd
            || accounted.tokens > reserved.tokens
            || accounted.requests > reserved.requests
        {
            tx.execute("UPDATE runs SET state='failed',epoch=epoch+1,error='status query exceeded reservation' WHERE id=?1 AND epoch=?2 AND state IN ('running','waiting_provider')",params![run,epoch])?;
        }
        Ok(())
    }
}
