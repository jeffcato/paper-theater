"""Publish an immutable diagnostic readiness revision using existing source and PCM only."""
import argparse
import array
import copy
import io
import json
import math
import os
from pathlib import Path
import re
import sys
import tempfile
import wave
from capture_inventory import canonical,digest,load,read

BASE='fixtures/shared/two-scene-v1'
VERSION='two-scene-readiness-v2'


def encoded(value):
    return (json.dumps(value,ensure_ascii=False,indent=2,allow_nan=False)+'\n').encode()


def _source_sample(sample,mapping):
    for segment in mapping['segments']:
        a,b=segment['destination'];c,d=segment['source']
        if a<=sample<=b:
            numerator=(sample-a)*(d-c)
            if numerator%(b-a):raise ValueError('nonintegral source sample')
            return c+numerator//(b-a)
    raise ValueError('measurement lies in a presentation gap')


def prepare_revision(root):
    root=Path(root)
    freeze=load(root,'docs/implementation/integration/contracts-ready.json')
    index=read(root,'docs/implementation/integration/fixture-index.json')
    if digest(index)!=freeze['fixture_sha256']:raise ValueError('base freeze index changed')
    for record in json.loads(index):
        data=read(root,BASE+'/'+record['path'])
        if len(data)!=record['bytes'] or digest(data)!=record['sha256']:raise ValueError('frozen base bytes changed')
    original=load(root,BASE+'/fixture.json');doc=copy.deepcopy(original)
    timeline=doc['timeline'];old_map=original['presentation_map'];words=doc['read_along']['words']
    old_transition=original['timeline']['transitions'][0]
    incoming=timeline['scenes'][1];groups={g['id']:g for g in doc['manifest']['reveals']}
    required=max(groups[id]['not_before_sample'] for id in incoming['reveal_group_ids'])
    # The earliest sentence ending after the required fact; cut only within its measured word gap.
    choices=[(i,w) for i,w in enumerate(words[:-1]) if w['end_sample']>=required and re.match(r'^[.!?][”\"\']?\s',w['following_text'])]
    if not choices:raise ValueError('no measured sentence pause after required facts')
    i,last_word=choices[0];next_word=words[i+1]
    lo=_source_sample(last_word['end_sample'],old_map);hi=_source_sample(next_word['start_sample'],old_map)
    if hi-lo<2:raise ValueError('no positive measured sentence gap')
    source_wav=read(root,'fixtures/shared/source-media/audio.wav')
    if digest(source_wav)!=old_map['source_audio_sha256']:raise ValueError('source audio hash mismatch')
    with wave.open(io.BytesIO(source_wav)) as wav:
        if (wav.getframerate(),wav.getnchannels(),wav.getsampwidth())!=(24000,1,2):raise ValueError('unexpected source format')
        mono=array.array('h');mono.frombytes(wav.readframes(wav.getnframes()))
    if sys.byteorder!='little':mono.byteswap()
    midpoint=(lo+hi)//2
    zeros=[s for s in range(lo+1,hi) if mono[s]==0]
    if not zeros:raise ValueError('no exact zero sample in measured sentence gap')
    cut=min(zeros,key=lambda s:(abs(s-midpoint),s))
    radius=120;peak=max(abs(s) for s in mono[max(lo,cut-radius):min(hi,cut+radius)])
    offset=old_map['segments'][0]['destination'][0]
    gap=old_transition['fade_in_end']-old_transition['fade_out_start']
    if gap!=192000:raise ValueError('unexpected formal transition length')
    fade=offset+cut*2
    def mapped(source):return offset+source*2+(gap if source>=cut else 0)
    mapping=copy.deepcopy(old_map)
    mapping['segments']=[{'source':[0,cut],'destination':[offset,fade]},
                         {'source':[cut,len(mono)],'destination':[fade+gap,timeline['closing']['last_prose_action_sample']]}]
    mapping['gaps']=[old_map['gaps'][0],[fade,fade+gap],old_map['gaps'][-1]]
    for collection in [doc['read_along']['words'],doc['read_along']['groups']]:
        for item in collection:
            a=_source_sample(item['start_sample'],old_map);b=_source_sample(item['end_sample'],old_map)
            if a<cut<b:raise ValueError('cut crosses a measured word/highlight interval')
            item['start_sample']=mapped(a);item['end_sample']=mapped(b)
    for group in doc['manifest']['reveals']:
        group['not_before_sample']=mapped(_source_sample(group['not_before_sample'],old_map))
    tr=timeline['transitions'][0]
    tr.update(fade_out_start=fade,swap_sample=fade+48000,fade_in_start=fade+144000,fade_in_end=fade+gap)
    timeline['scenes'][0]['narration'][1]=fade;incoming['narration'][0]=fade+gap
    boundary=next_word['source_span'][0]
    timeline['scenes'][0]['source_span'][1]=boundary;incoming['source_span'][0]=boundary
    cue_revisions=[]
    for cue in timeline['cues']:
        if cue['scene_id']==incoming['id'] and cue['start_sample']<incoming['narration'][0]:
            before=[cue['start_sample'],cue['end_sample']];duration=before[1]-before[0]
            cue['start_sample']=incoming['narration'][0];cue['end_sample']=cue['start_sample']+duration
            cue_revisions.append({'id':cue['id'],'before':before,'after':[cue['start_sample'],cue['end_sample']],
                                  'reason':'Explicit diagnostic revision: retain the same action/target/duration at the incoming full-picture entrance.'})
    with wave.open(io.BytesIO(read(root,BASE+'/audio/master.wav'))) as wav:
        if (wav.getframerate(),wav.getnchannels(),wav.getsampwidth())!=(48000,2,2):raise ValueError('unexpected master format')
        pcm=wav.readframes(wav.getnframes())
    a,b=old_transition['fade_out_start'],old_transition['fade_in_end']
    if any(pcm[a*4:b*4]):raise ValueError('old transition contains nonzero audio')
    continuous=pcm[:a*4]+pcm[b*4:]
    new_pcm=continuous[:fade*4]+bytes(gap*4)+continuous[fade*4:]
    if len(new_pcm)!=len(pcm):raise ValueError('master duration changed')
    output=io.BytesIO()
    with wave.open(output,'wb') as wav:
        wav.setparams((2,2,48000,timeline['sample_count'],'NONE','not compressed'));wav.writeframes(new_pcm)
    files={r['path']:read(root,BASE+'/'+r['path']) for r in load(root,BASE+'/inventory.json')}
    files['audio/master.wav']=output.getvalue();audio_sha=digest(files['audio/master.wav'])
    files['presentation-map.json']=encoded(mapping);map_sha=digest(files['presentation-map.json'])
    doc['presentation_map']=mapping
    timeline['delivery_audio_sha256']=audio_sha;timeline['presentation_map_sha256']=map_sha
    doc['read_along']['delivery_audio_sha256']=audio_sha;doc['read_along']['presentation_map_sha256']=map_sha
    recipe_sha=digest(read(root,'tools/generation/readiness_correction.py'))
    for attestation in doc['acceptance']['attestations']:
        attestation['presentation_map_sha256']=map_sha;attestation['delivery_audio_sha256']=audio_sha
        attestation['recipe_sha256']=recipe_sha
        if attestation['kind']!='image_scope':attestation['target_sha256']=audio_sha
        if attestation['measured_end_sample'] is not None:
            attestation['measured_end_sample']=mapped(_source_sample(attestation['measured_end_sample'],old_map))
    proposal={'schema_version':1,'revision_id':VERSION,'provenance':'diagnostic','production_approved':False,'provider_calls':0,
        'parent_fixture_sha256':freeze['fixture_sha256'],'parent_manifest_sha256':freeze['manifest_sha256'],
        'recipe_sha256':recipe_sha,'problem':{'figure_id':'em-2','old_fade_in_start':old_transition['fade_in_start'],
            'old_eligible_sample':required,'late_samples':required-old_transition['fade_in_start'],'late_seconds':(required-old_transition['fade_in_start'])/48000},
        'decision':'Move the formal transition after the complete source sentence ending in hole; retain exact source PCM and the narration-free 1s fade / 2s dark hold / 1s fade.',
        'sentence_pause':{'last_word_id':last_word['id'],'next_word_id':next_word['id'],'source_gap':[lo,hi],
            'cut_source_sample':cut,'cut_value_pcm16':int(mono[cut]),'local_window_peak_pcm16':peak,
            'window_radius_samples':radius,'local_window_peak_dbfs':20*math.log10(peak/32768) if peak else None,
            'scope':'Local edit observation only; production join/listening review remains missing.'},
        'transition_before':old_transition,'transition_after':copy.deepcopy(tr),'source_partition':[boundary],
        'cue_revisions':cue_revisions,'unchanged':['verbatim source','source audio bytes','raw alignment bytes','176 word identities','170 group memberships','profile/art bytes','sample count','closing phases'],
        'new_bindings':{'delivery_audio_sha256':audio_sha,'presentation_map_sha256':map_sha},
        'incoming_readiness':{'scene_id':incoming['id'],'eligible_sample':groups[incoming['reveal_group_ids'][0]]['not_before_sample'],
            'fade_in_start':tr['fade_in_start'],'outcome':'pass','scope':'Deterministic diagnostic schedule only; synthetic evidence is not production approval.'},
        'pending':['production acoustic/image/fact acceptance','narrative/readability review of revised hold','audio join and mix review','actual browser capture','nod-em rendered motion','rendering profile/device acceptance']}
    if proposal['incoming_readiness']['eligible_sample']>tr['fade_in_start']:raise ValueError('revision still has an unready entrance')
    files['correction.json']=encoded(proposal);files['evidence/narration.wav']=source_wav
    credits=load(root,BASE+'/credits.json')
    credits['transformation']+='; readiness v2 relocates the existing 192000-sample silent transition into a measured sentence pause; every retained source PCM sample is unchanged.'
    credits['limitations']+=['v2 join/narrative/rendered-motion reviews remain pending']
    files['credits.json']=encoded(credits)
    files['timeline.json']=encoded(timeline);files['read-along.json']=encoded(doc['read_along'])
    media={r['path']:r for r in load(root,BASE+'/inventory.json')}
    def reference(path):
        ref=copy.deepcopy(media.get(path,{'path':path,'media_type':'audio/wav' if path.endswith('.wav') else 'application/json'}))
        ref.update(sha256=digest(files[path]),bytes=len(files[path]));return ref
    for name,ref in doc['manifest']['entries'].items():doc['manifest']['entries'][name]=reference(ref['path'])
    doc['manifest']['assets']=[reference(r['path']) for r in doc['manifest']['assets']]+[reference('correction.json'),reference('evidence/narration.wav')]
    inventory=sorted(list(doc['manifest']['entries'].values())+doc['manifest']['assets'],key=lambda r:r['path'])
    files['inventory.json']=encoded(inventory)
    acceptance=doc['acceptance'];acceptance['inventory_sha256']=digest(files['inventory.json']);acceptance['recipe_sha256']=recipe_sha
    for id in ['readiness-v2-audio-join-review','readiness-v2-narrative-render-review']:
        acceptance['required_check_ids'].append(id)
        acceptance['findings'].append({'id':id,'field':'acceptance','outcome':'missing','message':'New diagnostic presentation revision requires actual review.','evidence':[]})
    files['acceptance.json']=encoded(acceptance)
    doc['manifest']['acceptance']={'path':'acceptance.json','sha256':digest(files['acceptance.json']),'bytes':len(files['acceptance.json']),'media_type':'application/json'}
    doc['manifest']['chapter_label']='Saved partial excerpt / readiness v2'
    files['manifest.json']=encoded(doc['manifest']);files['fixture.json']=encoded(doc)
    index=[{'path':p,'bytes':len(b),'sha256':digest(b)} for p,b in sorted(files.items())]
    files['file-index.json']=canonical(index)
    record={'schema_version':1,'revision_id':VERSION,'status':'published-diagnostic-correction','provenance':'diagnostic',
        'production_approved':False,'provider_calls':0,'parent_fixture_path':BASE,'parent_fixture_sha256':freeze['fixture_sha256'],
        'parent_manifest_sha256':freeze['manifest_sha256'],'fixture_sha256':digest(files['file-index.json']),
        'manifest_sha256':digest(files['manifest.json']),'inventory_sha256':digest(files['inventory.json']),
        'profile_sha256':doc['acceptance']['profile_sha256'],'recipe_sha256':recipe_sha,
        'note':'Separate immutable diagnostic fixture; no production catalog activation or acceptance.'}
    files['revision.json']=encoded(record)
    return {'document':doc,'proposal':proposal,'record':record,'files':files}


def publish_revision(root,output):
    root=Path(root).resolve();output=Path(output).resolve()
    if not output.is_relative_to(root/'fixtures/generation'):raise ValueError('output must remain in generation fixtures')
    revision=prepare_revision(root)
    if output.exists():
        if any(p.is_symlink() for p in output.rglob('*')):raise ValueError('symlink in published version')
        actual={p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_file()}
        if actual!=set(revision['files']):raise ValueError('published version inventory changed')
        for path,data in revision['files'].items():
            if (output/path).read_bytes()!=data:raise ValueError('immutable version already exists; choose a new version')
    else:
        output.parent.mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(dir=output.parent,prefix='.readiness-') as folder:
            stage=Path(folder)/output.name;stage.mkdir()
            for path,data in revision['files'].items():
                destination=stage/path;destination.parent.mkdir(parents=True,exist_ok=True);destination.write_bytes(data)
            os.rename(stage,output)
    return revision['record']


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    root=Path(__file__).resolve().parents[2]
    record=publish_revision(root,args.output)
    print(f"Published {VERSION}: manifest {record['manifest_sha256']}; diagnostic; provider calls: 0")

if __name__=='__main__':main()
