"""Prepare diagnostic capture requests from the frozen fixture; no rendering or dispatch."""
import argparse
import hashlib
import json
from pathlib import Path

FIXTURE='fixtures/shared/two-scene-v1'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()+b'\n'


def read(root,path):
    relative=Path(path)
    if relative.is_absolute() or '..' in relative.parts:raise ValueError('unsafe inventory path')
    current=root
    for part in relative.parts:
        current=current/part
        if current.is_symlink():raise ValueError('symlink in frozen inputs')
    return current.read_bytes()


def load(root,path):
    return json.loads(read(root,path))


def snapshot(root,paths):
    return [{'path':p,'bytes':len(data),'sha256':digest(data)} for p in sorted(paths) for data in [read(root,p)]]


def build_inventory(root,*,fixture_path=FIXTURE):
    root=Path(root)
    freeze=load(root,'docs/implementation/integration/contracts-ready.json')
    index=read(root,'docs/implementation/integration/fixture-index.json')
    if digest(index)!=freeze['fixture_sha256']:raise ValueError('frozen fixture index mismatch')
    for entry in json.loads(index):
        raw=read(root,FIXTURE+'/'+entry['path'])
        if len(raw)!=entry['bytes'] or digest(raw)!=entry['sha256']:raise ValueError('frozen fixture bytes changed: '+entry['path'])
    parent_fixture_sha=freeze['fixture_sha256']
    if fixture_path!=FIXTURE:
        revision=load(root,fixture_path+'/revision.json')
        if (revision['parent_fixture_sha256']!=parent_fixture_sha
                or revision['provenance']!='diagnostic' or revision['production_approved'] is not False):raise ValueError('unrecognized diagnostic parent revision')
        revised_index=read(root,fixture_path+'/file-index.json')
        if digest(revised_index)!=revision['fixture_sha256']:raise ValueError('revision index mismatch')
        for entry in json.loads(revised_index):
            raw=read(root,fixture_path+'/'+entry['path'])
            if len(raw)!=entry['bytes'] or digest(raw)!=entry['sha256']:raise ValueError('revision payload changed')
        freeze=dict(freeze,fixture_sha256=revision['fixture_sha256'],manifest_sha256=revision['manifest_sha256'])
    manifest=load(root,fixture_path+'/manifest.json')
    if digest(read(root,fixture_path+'/manifest.json'))!=freeze['manifest_sha256']:raise ValueError('frozen manifest changed')
    doc=load(root,fixture_path+'/fixture.json');timeline=doc['timeline'];profile=load(root,fixture_path+'/profile.json')
    if doc['manifest']!=manifest or doc['shell']['profile']!=profile:raise ValueError('fixture envelope/profile mismatch')
    n=timeline['sample_count'];rate=timeline['sample_rate'];opening=timeline['opening'];closing=timeline['closing']
    if type(n)!=int or not 0<n<=9007199254740991 or rate!=48000:raise ValueError('invalid delivery domain')
    player_paths=[p.relative_to(root).as_posix() for p in (root/'packages/player/dist/src').glob('*.js')]
    if not player_paths:raise ValueError('player build required before pinning capture code')
    player_files=snapshot(root,player_paths)
    source_files=snapshot(root,[p.relative_to(root).as_posix() for p in (root/'packages/player/src').glob('*.ts')])
    checker_path='tools/generation/capture_inventory.py';checker_sha=digest(read(root,checker_path))
    bindings={
        'contract_revision':freeze['contract_revision'],'fixture_sha256':freeze['fixture_sha256'],
        'parent_fixture_sha256':parent_fixture_sha,
        'manifest_sha256':freeze['manifest_sha256'],'profile_id':profile['id'],
        'profile_sha256':digest(read(root,fixture_path+'/profile.json')),'profile_status':profile['status'],
        'timeline_sha256':digest(read(root,fixture_path+'/timeline.json')),
        'delivery_audio_sha256':manifest['entries']['audio']['sha256'],
        'player_sha256':digest(canonical(player_files)),'player_files':player_files,
        'player_sources_sha256':digest(canonical(source_files)),'player_sources':source_files,
        'checker_sha256':checker_sha,'checker_path':checker_path,
        'digest_recipe':'SHA-256 of sorted-path file records encoded as UTF-8 sorted-key compact JSON plus LF',
        'player_binding_scope':'Existing compiled player JS snapshot; capture must verify these bytes and record actual browser/bundle identities.',
        'checker_binding_scope':'Sampling inventory generator; rendered review checker remains unexecuted.'}
    frames={};boundaries=[];holds=[];entries=[];actions=[]
    def frame(sample,reason):
        if type(sample)!=int or not 0<=sample<=n:raise ValueError('sample outside delivery domain')
        frames.setdefault(sample,set()).add(reason)
        return 'sample:'+str(sample)
    def boundary(id,sample):
        points=[s for s in [sample-1,sample,sample+1] if 0<=s<=n]
        if type(sample)!=int or not 0<=sample<=n:raise ValueError('boundary outside delivery domain')
        boundaries.append({'id':id,'sample':sample,'frame_ids':[frame(s,id) for s in points]})
    def hold(id,kind,interval):
        a,b=interval
        if not 0<=a<b<=n:raise ValueError('invalid authored hold')
        points=sorted({a,(a+b)//2,b-1})
        holds.append({'id':id,'kind':kind,'interval':[a,b],'duration_samples':b-a,'duration_seconds':(b-a)/rate,
                      'frame_ids':[frame(s,id) for s in points],
                      'evidence_required':'Duration-labeled sequence plus actual observations; sampled stills alone do not certify the entire interval.'})
    for name in ['title_hold','title_dismiss','curtain_open']:
        a,b=opening[name];boundary('opening:'+name+':start',a);boundary('opening:'+name+':end',b)
        hold('opening:'+name,name,[a,b])
    boundary('opening:first-prose',opening['first_prose_sample'])
    boundary('opening:first-picture-eligible',opening['first_picture_eligible_sample'])
    if opening.get('intro_narration'):
        hold('opening:closed-curtain-context','intro_narration',opening['intro_narration'])
    groups={g['id']:g for g in manifest['reveals']}
    for group in groups.values():
        if group['not_before_sample'] is not None:boundary('reveal:'+group['id'],group['not_before_sample'])
    for i,tr in enumerate(timeline['transitions']):
        for name in ['fade_out_start','swap_sample','fade_in_start','fade_in_end']:boundary(f'transition:{i}:{name}',tr[name])
        hold(f'transition:{i}:dark-hold','dark_hold',[tr['swap_sample'],tr['fade_in_start']])
    for i,scene in enumerate(timeline['scenes']):
        id=scene['id'];a,b=scene['narration'];boundary('scene:'+id+':narration-start',a);boundary('scene:'+id+':narration-end',b)
        tr=next((t for t in timeline['transitions'] if t['to_scene_id']==id),None)
        entry=tr['fade_in_start'] if tr else opening['curtain_open'][0]
        picture=tr['fade_in_end'] if tr else opening['curtain_open'][1]
        required=[inst['id'] for inst in scene['entry_snapshot']['instances']]
        scoped=[groups[g] for g in scene['reveal_group_ids']]
        eligible={c for g in scoped if g['not_before_sample'] is not None and g['not_before_sample']<=entry for c in g['contributor_ids']}
        entries.append({'scene_id':id,'entry_sample':entry,'full_picture_sample':picture,
            'required_contributor_ids':required,'eligible_at_entry':sorted(eligible),
            'readiness':'pass' if set(required)<=eligible else 'fail',
            'readiness_scope':'Diagnostic authored eligibility only; no production evidence is granted.',
            'frame_ids':[frame(entry,'entry:'+id),frame(picture,'full-picture:'+id)]})
        hold('scene:'+id+':narrative-picture','narrative_picture',[max(picture,a),b])
    for cue in timeline['cues']:
        id=cue['id'];a,b=cue['start_sample'],cue['end_sample']
        if not 0<=a<b<=n:raise ValueError('invalid action interval')
        boundary('action:'+id+':start',a);boundary('action:'+id+':end',b)
        interior=sorted({a+(b-a)*q//4 for q in [1,2,3]})
        points=sorted({max(0,a-1),min(b-1,a+1),*interior,b-1,b})
        actions.append({'id':id,'scene_id':cue['scene_id'],'target_id':cue['target_id'],
            'action':cue['action'],'params':cue['params'],'interval':[a,b],
            'duration_samples':b-a,'duration_seconds':(b-a)/rate,'interior_samples':interior,
            'frame_ids':[frame(s,'action:'+id) for s in points],
            'motion_pair':[frame(min(b-1,a+1),'motion-pair:'+id),frame((a+b)//2,'motion-pair:'+id)],
            'clip_interval':[max(0,a-rate//4),min(n,b+rate//4)],
            'evidence_required':'Compare target pose and actual pixels through the action; a cue label or state digest alone is insufficient. No motion is a failed action capture.'})
    boundary('closing:last-prose-action',closing['last_prose_action_sample'])
    for label,value in [('start',closing['curtain_close'][0]),('end',closing['curtain_close'][1])]:boundary('closing:curtain:'+label,value)
    hold('closing:curtain-close','curtain_close',closing['curtain_close'])
    if closing.get('score_tail'):hold('closing:score-tail','score_tail',closing['score_tail'])
    boundary('terminal',n)
    context={'manifest_digest':bindings['manifest_sha256'],'candidate_id':'frozen-fixture:'+bindings['fixture_sha256'],
             'checker_sha256':checker_sha,'player_sha256':bindings['player_sha256'],'profile_id':profile['id']}
    viewports=[{'id':f'{w}x{h}','width':w,'height':h,'dpr':1} for w,h in [(960,540),(1280,720),(1920,1080)]]
    frame_list=[{'id':'sample:'+str(s),'sample':s,'seconds':s/rate,'reason_ids':sorted(reasons)} for s,reasons in sorted(frames.items())]
    requests=[{'id':f['id']+':'+v['id'],'frame_id':f['id'],'sample':f['sample'],'viewport_id':v['id'],
               'request':dict(context,sample=f['sample'],width=v['width'],height=v['height'])} for f in frame_list for v in viewports]
    return {'schema_version':1,'kind':'diagnostic-capture-sampling-inventory','production_approved':False,'provider_calls':0,
        'fixture_path':fixture_path,'sample_rate':rate,'sample_count':n,'bindings':bindings,
        'interval_policy':'half-open [start,end); terminal sample_count is an explicit ended-state evaluation',
        'capture_status':'planned; no browser images, rendered review or production acceptance claimed',
        'opening':opening,'reveals':[{**g,'scope':'diagnostic eligibility only; actual visibility depends on curtain/scene/filtering and remains to be captured'} for g in groups.values()],
        'transitions':timeline['transitions'],
        'boundaries':boundaries,'holds':holds,'entries':entries,'actions':actions,
        'terminal':{'sample':n,'frame_id':'sample:'+str(n),'expected_curtain_open_ratio':0,
                    'snapshot_sha256':digest(canonical(timeline['terminal_snapshot'])),
                    'snapshot_digest_recipe':'UTF-8 sorted-key compact JSON plus LF',
                    'expected_instance_ids':[i['id'] for i in timeline['terminal_snapshot']['instances']]},
        'viewports':viewports,'frames':frame_list,'requests':requests,
        'seek_sequences':[{'id':'backward-reveal:'+g['id'],'samples':[n,g['not_before_sample']-1,g['not_before_sample']]} for g in groups.values() if g['not_before_sample'] is not None and g['not_before_sample']>0],
        'capture_requirements':[
            'Verify frozen fixture and pinned player/checker/profile bytes before capture; stop on a changed binding.',
            'Use the diagnostic player entry point and the sampled media playhead for every scene, caption and highlight.',
            'Return request identities, evaluated state/contributors and actual PNG hashes, dimensions and browser/bundle/device identities.',
            'Measure aperture blackout including emissive pixels and shadows while checking shell/text readability.',
            'Compare before/at/after reveal contributors, incoming-picture readiness, paired action motion and exact terminal state.',
            'Keep synthetic/imported diagnostic reveal assertions separate from production fact acceptance; all creative findings remain pending.',
            'Repeat backward seeks without leaking later contributors. Missing captures or unchanged required motion cannot pass review.']}


def validate_inventory(root,inventory):
    if inventory!=build_inventory(root,fixture_path=inventory.get('fixture_path',FIXTURE)):
        raise ValueError('capture inventory is incomplete, altered or bound to changed inputs/code')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--fixture',default=FIXTURE)
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[2]
    inventory=build_inventory(root,fixture_path=args.fixture)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(inventory,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
    print(f"Prepared {len(inventory['frames'])} samples / {len(inventory['requests'])} requests; provider calls: 0")

if __name__=='__main__':main()
