use super::store::*;
use async_trait::async_trait;
use pt_contracts as c;
use rusqlite::{params,OptionalExtension};
use serde_json::json;
use std::{collections::HashMap,sync::{Arc,Mutex}};

#[derive(Clone)]
pub struct Services { store:Store, run:String, epoch:i64, mode:c::ExecutionMode, candidates:Arc<Mutex<HashMap<String,(c::CandidateFile,Vec<u8>)>>>, proposals:Arc<Mutex<Vec<c::JournalProposal>>> }
fn service(e:Error)->c::ServiceError { if e.code=="stale_epoch" {c::ServiceError::Cancelled}else{c::ServiceError::Internal(e.to_string())} }
impl Services {
    fn active(&self)->std::result::Result<(),c::ServiceError> {Store::active(&self.store.db().map_err(service)?,&self.run,self.epoch).map_err(service)}
}
#[async_trait]
impl c::RunServices for Services {
    async fn check_cancelled(&self)->std::result::Result<(),c::ServiceError>{self.active()}
    async fn read_artifact(&self,a:&c::ArtifactRef)->std::result::Result<Vec<u8>,c::ServiceError>{self.active()?;let bytes=self.store.artifact(&a.id).map_err(service)?.0;a.file.verify(&bytes)?;Ok(bytes)}
    async fn write_candidate(&self,r:c::CandidateWrite,bytes:Vec<u8>)->std::result::Result<c::CandidateFile,c::ServiceError>{
        self.active()?;
        if !c::valid_id(&r.candidate_id) || !c::safe_path(&r.relative_path) || bytes.len() as u64>r.max_bytes || r.max_bytes>32*1024*1024 {return Err(c::ServiceError::Unauthorized("candidate bounds".into()));}
        let file=c::FileRef{path:r.relative_path,sha256:digest(&bytes),bytes:bytes.len() as u64,media_type:r.media_type,dimensions:None};file.validate()?;
        let result=c::CandidateFile{candidate_id:r.candidate_id,file,provenance:r.provenance};
        let mut candidates=self.candidates.lock().unwrap();
        if candidates.len()>=64 || candidates.values().map(|(_,b)|b.len()).sum::<usize>()+bytes.len()>32*1024*1024 {return Err(c::ServiceError::Limit("stage candidate size".into()));}
        if let Some((old,_))=candidates.get(&result.candidate_id) {if old.file!=result.file{return Err(c::ServiceError::Unauthorized("candidate identity reused".into()));}}
        candidates.insert(result.candidate_id.clone(),(result.clone(),bytes));Ok(result)
    }
    async fn emit(&self,entry:c::JournalProposal)->std::result::Result<(),c::ServiceError>{self.active()?;if entry.summary.len()>4000{return Err(c::ServiceError::Limit("journal summary".into()));}let mut proposals=self.proposals.lock().unwrap();if proposals.len()>=128{return Err(c::ServiceError::Limit("journal entries".into()));}proposals.push(entry);Ok(())}
    async fn provider(&self,_:c::ProviderOperation)->std::result::Result<c::ProviderResult,c::ServiceError>{self.active()?;c::require_live(self.mode)?;Err(c::ServiceError::Unauthorized("live transport integration/accounting pending".into()))}
    async fn helper(&self,_:c::HelperOperation)->std::result::Result<c::HelperResult,c::ServiceError>{self.active()?;Err(c::ServiceError::Unauthorized("helper recipe not registered".into()))}
    async fn library(&self,q:c::LibraryQuery)->std::result::Result<Vec<c::LibraryBinding>,c::ServiceError>{self.active()?;let db=self.store.db().map_err(service)?;let mut found=Vec::new();for entity in q.entity_ids {let saved:Option<String>=db.query_row("SELECT record FROM library WHERE book=?1 AND entity=?2",params![q.book_id,entity],|r|r.get(0)).optional().map_err(Error::from).map_err(service)?;if let Some(saved)=saved {let binding:c::LibraryBinding=serde_json::from_str(&saved).map_err(Error::from).map_err(service)?;if binding.revision>q.revision{return Err(c::ServiceError::RevisionConflict);}found.push(binding);}}Ok(found)}
}
#[derive(Clone)]
pub struct Runtime { pub store:Store, executor:Arc<dyn c::StageExecutor>, pub fixture_delay_ms:u64, saved_inputs:Option<(std::path::PathBuf,std::path::PathBuf)> }
impl Runtime {
    pub fn new(store:Store,executor:Arc<dyn c::StageExecutor>)->Self{Self{store,executor,fixture_delay_ms:0,saved_inputs:None}}
    pub fn with_saved_inputs(mut self,fixture:std::path::PathBuf,sources:std::path::PathBuf)->Self{self.saved_inputs=Some((fixture,sources));self}
    pub async fn work_once(&self)->Result<bool>{self.store.wake_due()?;let Some((run,epoch))=self.store.claim()? else{return Ok(false)};if self.store.snapshot(&run)?["request"]["pins"]["recipe_sha256"]==digest(b"saved-diagnostic-v1") {
            let prepared=match &self.saved_inputs {Some((fixture,sources))=>self.store.seed_fixture(&run,epoch,fixture,Some(sources)),None=>fail("fixture_unavailable","saved fixture inputs are not configured")};
            if let Err(e)=prepared{self.store.finish(&run,epoch,"failed",Some(e.code))?;return Err(e);}
        }
        self.execute_claimed(&run,epoch).await?;Ok(true)}
    pub async fn execute_claimed(&self,run:&str,epoch:i64)->Result<()>{
        let mut result=Ok(());
        for stage in c::STAGES {if let Err(e)=self.execute_stage(run,epoch,stage).await {result=Err(e);break;}}
        if self.store.is_active(run,epoch)? {match &result {Ok(())=>self.store.finish(run,epoch,"succeeded",None)?,Err(e)=>self.store.finish(run,epoch,"failed",Some(e.code))?,}}
        if result.as_ref().is_err_and(|e|e.code=="stale_epoch"){return Ok(());}result
    }
    pub async fn execute_stage(&self,run:&str,epoch:i64,stage:c::Stage)->Result<Vec<c::ArtifactRef>>{
        Store::active(&self.store.db()?,run,epoch)?;
        let snapshot=self.store.snapshot(run)?;let request:c::RunRequest=serde_json::from_value(snapshot["request"].clone())?;
        let stage_name=serde_json::to_value(stage)?.as_str().unwrap().to_owned();
        let mut inputs=self.store.seed_inputs(run)?;let db=self.store.db()?;
        for prior in c::STAGES {if prior==stage{break;}let name=serde_json::to_value(prior)?.as_str().unwrap().to_owned();let saved:Option<String>=db.query_row("SELECT output FROM units WHERE run=?1 AND stage=?2 ORDER BY rowid DESC LIMIT 1",params![run,name],|r|r.get(0)).optional()?;if let Some(saved)=saved{inputs.extend(serde_json::from_str::<Vec<c::ArtifactRef>>(&saved)?);}}
        let input_hash=digest(&serde_json::to_vec(&json!({"request":snapshot["request"],"inputs":inputs,"stage":stage,"contracts":c::CONTRACT_REVISION,"services":c::SERVICE_REVISION}))?);
        let key=digest(format!("{stage_name}:{input_hash}").as_bytes());
        if let Some(saved)=self.store.cached(run,&stage_name,&key,&input_hash)? {self.store.transaction(|tx|{Store::active(tx,run,epoch)?;Store::emit(tx,run,"cache_reused",&json!({"stage":stage_name,"unit_key":key,"artifact_ids":saved.iter().map(|a|&a.id).collect::<Vec<_>>()}))?;Ok(())})?;return Ok(saved);}
        let input=c::StageInput{schema_version:1,run_id:run.into(),stage,unit_id:key.clone(),epoch:epoch as u64,revision:0,inputs,pins:request.pins,mode:request.mode,limits:request.limits,library_revision:0};input.validate().map_err(|e|Error::new("invalid_input",e.to_string()))?;
        self.store.transaction(|tx|{Store::active(tx,run,epoch)?;tx.execute("UPDATE stages SET state='running' WHERE run=?1 AND name=?2",params![run,stage_name])?;Store::emit(tx,run,"task_started",&json!({"stage":stage_name,"unit_key":key}))?;Ok(())})?;
        let services=Services{store:self.store.clone(),run:run.into(),epoch,mode:request.mode,candidates:Arc::new(Mutex::new(HashMap::new())),proposals:Arc::new(Mutex::new(Vec::new()))};
        if self.fixture_delay_ms>0{tokio::time::sleep(std::time::Duration::from_millis(self.fixture_delay_ms)).await;}
        let output=self.executor.execute(input,&services).await.map_err(|e|Error::new("stage_failed",e.to_string()))?;
        let outcome=output.outcome;
        let artifacts=self.store.commit_shared(run,epoch,&stage_name,&key,&input_hash,output,&services)?;
        if outcome!=c::StageOutcome::Complete {return fail("stage_needs_repair","diagnostic artifacts preserved; required evidence remains unresolved");}
        Ok(artifacts)
    }
}
impl Store {
    fn cached(&self,run:&str,stage:&str,key:&str,input_hash:&str)->Result<Option<Vec<c::ArtifactRef>>>{
        let saved:Option<(String,String)>=self.db()?.query_row("SELECT input_hash,output FROM units WHERE run=?1 AND stage=?2 AND unit_key=?3 AND outcome='complete'",params![run,stage,key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((hash,output))=saved{if hash!=input_hash{return fail("stale_unit","unit inputs changed");}let artifacts:Vec<c::ArtifactRef>=serde_json::from_str(&output)?;for a in &artifacts {a.file.verify(&self.artifact(&a.id)?.0).map_err(|e|Error::new("corrupt_artifact",e.to_string()))?;}return Ok(Some(artifacts));}Ok(None)
    }
    fn commit_shared(&self,run:&str,epoch:i64,stage:&str,key:&str,input_hash:&str,output:c::StageOutput,services:&Services)->Result<Vec<c::ArtifactRef>>{
        if output.artifacts.len()>64 || output.decisions.len()>64 || output.findings.len()>128{return fail("stage_boundary","bounded output required; shared package activation pending R2-005");}
        let mut artifacts=Vec::new();
        {let candidates=services.candidates.lock().unwrap();for a in &output.artifacts {let (stored,bytes)=candidates.get(&a.candidate_id).ok_or_else(||Error::new("missing_candidate","candidate was not written through services"))?;if stored.file!=a.file || stored.provenance!=a.provenance{return fail("invalid_candidate","returned candidate differs from written bytes");}a.file.verify(bytes).map_err(|e|Error::new("invalid_candidate",e.to_string()))?;self.put(bytes)?;artifacts.push(c::ArtifactRef{id:digest(format!("{run}:{key}:{}:{}",a.file.path,a.file.sha256).as_bytes()),file:a.file.clone(),provenance:a.provenance});}}
        let snapshot=self.snapshot(run)?;let pins:c::Pins=serde_json::from_value(snapshot["request"]["pins"].clone())?;
        let candidates=services.candidates.lock().unwrap().values().map(|(f,b)|(f.file.path.clone(),b.clone())).collect::<std::collections::BTreeMap<_,_>>();
        let mapping=super::fixture::candidate_mapping(&output,&self.seed_inputs(run)?,&artifacts,&candidates,&pins)?;
        self.transaction(|tx|{
            Store::active(tx,run,epoch)?;
            for a in &artifacts {tx.execute("INSERT INTO artifacts(id,run,name,sha256,bytes,media_type,provenance,reference) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![a.id,run,a.file.path,a.file.sha256,a.file.bytes,a.file.media_type,serde_json::to_value(a.provenance)?.as_str(),serde_json::to_string(a)?])?;Store::emit(tx,run,"artifact_ready",&json!({"stage":stage,"unit_key":key,"artifact":a}))?;}
            for d in &output.decisions {if d.summary.is_empty() || d.summary.len()>4000 || d.inputs.is_empty(){return fail("invalid_decision","source/input references and recorded explanation required");}for a in &d.inputs{self.artifact(&a.id)?;}Store::emit(tx,run,"director_decision",&serde_json::to_value(d)?)?;}
            for f in &output.findings{Store::emit(tx,run,"check_completed",&serde_json::to_value(f)?)?;}
            for update in &output.library_updates {if update.artifacts.is_empty(){return fail("invalid_library","binding needs verified artifact refs");}let old:Option<i64>=tx.query_row("SELECT revision FROM library WHERE book=?1 AND entity=?2",params![update.book_id,update.entity_id],|r|r.get(0)).optional()?;if old.unwrap_or(0) as u64!=update.expected_revision{return fail("library_conflict","binding revision changed");}for a in &update.artifacts{let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=?1 AND sha256=?2)",params![a.id,a.file.sha256],|r|r.get(0))?;if !known{return fail("invalid_library","unknown binding artifact");}}
                let binding=c::LibraryBinding{book_id:update.book_id.clone(),entity_id:update.entity_id.clone(),binding:update.binding,revision:update.expected_revision+1,artifacts:update.artifacts.clone()};tx.execute("INSERT INTO library VALUES(?1,?2,?3,?4) ON CONFLICT(book,entity) DO UPDATE SET revision=excluded.revision,record=excluded.record",params![update.book_id,update.entity_id,binding.revision,serde_json::to_string(&binding)?])?;Store::emit(tx,run,"library_bound",&serde_json::to_value(binding)?)?;
            }
            for proposal in services.proposals.lock().unwrap().iter(){Store::emit(tx,run,"journal_entry",&serde_json::to_value(proposal)?)?;}
            if let Some((manifest_hash,mapping))=&mapping {tx.execute("INSERT INTO diagnostic_packages VALUES(?1,?2,?3,?4,?5) ON CONFLICT(run) DO UPDATE SET epoch=excluded.epoch,manifest_hash=excluded.manifest_hash,mapping=excluded.mapping,candidate=excluded.candidate",params![run,epoch,manifest_hash,serde_json::to_string(mapping)?,serde_json::to_string(&output.package_candidate)?])?;Store::emit(tx,run,"diagnostic_package_ready",&json!({"manifest_sha256":manifest_hash,"production_acceptance":"unresolved"}))?;}
            let complete=output.outcome==c::StageOutcome::Complete;let outcome=if complete{"complete"}else{"needs_repair"};
            tx.execute("INSERT INTO units VALUES(?1,?2,?3,?4,?5,?6)",params![run,stage,key,input_hash,serde_json::to_string(&artifacts)?,outcome])?;tx.execute("UPDATE stages SET state=?3 WHERE run=?1 AND name=?2",params![run,stage,if complete{"succeeded"}else{"repairing"}])?;Store::emit(tx,run,"task_completed",&json!({"stage":stage,"unit_key":key,"artifact_ids":artifacts.iter().map(|a|&a.id).collect::<Vec<_>>()}))?;Ok(artifacts)
        })
    }
    pub fn wake_due(&self)->Result<()>{self.transaction(|tx|{let ids=tx.prepare("SELECT id FROM runs WHERE state='waiting_provider' AND retry_at<=?1")?.query_map([now()],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;for id in ids{tx.execute("UPDATE runs SET state='recovering' WHERE id=?1",[&id])?;Store::emit(tx,&id,"provider_wait_ended",&json!({}))?;}Ok(())})}
}
pub struct FixtureExecutor;
#[async_trait]
impl c::StageExecutor for FixtureExecutor {
    async fn execute(&self,input:c::StageInput,services:&dyn c::RunServices)->std::result::Result<c::StageOutput,c::StageError>{services.check_cancelled().await?;
        let name=serde_json::to_value(input.stage).unwrap().as_str().unwrap().to_owned();
        let bytes=serde_json::to_vec(&json!({"schema_version":1,"provenance":"synthetic","diagnostic":true,"audience_eligible":false,"stage":input.stage,"missing_acceptance":["actual-source-media","independent-review","render-evidence"],"note":"Structural fixture; no creative decisions or measured work are claimed."})).unwrap();
        let artifact=services.write_candidate(c::CandidateWrite{candidate_id:input.unit_id,relative_path:format!("{name}-fixture.json"),media_type:"application/json".into(),max_bytes:4096,provenance:c::Provenance::Synthetic},bytes).await?;
        Ok(c::StageOutput{outcome:c::StageOutcome::Complete,artifacts:vec![artifact],decisions:vec![],findings:vec![],cache_keys:vec![],library_updates:vec![],package_candidate:None})
    }
}

/// The server supplies generation through its shared trait; runtime never imports its sibling.
pub struct FixturePipeline {generation:Arc<dyn c::StageExecutor>}
impl FixturePipeline {pub fn new(generation:Arc<dyn c::StageExecutor>)->Self{Self{generation}}}
#[async_trait]
impl c::StageExecutor for FixturePipeline {
    async fn execute(&self,input:c::StageInput,services:&dyn c::RunServices)->std::result::Result<c::StageOutput,c::StageError>{
        if input.pins.recipe_sha256==digest(b"saved-diagnostic-v1") && matches!(input.stage,c::Stage::Source|c::Stage::Package){self.generation.execute(input,services).await}else{FixtureExecutor.execute(input,services).await}
    }
}
