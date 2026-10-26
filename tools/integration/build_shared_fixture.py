#!/usr/bin/env python3
"""Rebuild the diagnostic package exclusively from product-local saved bytes."""
from pathlib import Path
import array, copy, hashlib, json, math, struct, sys, wave
ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT/'fixtures/shared/source-media'
OUT = ROOT/'fixtures/shared/two-scene-v1'
def sha(b): return hashlib.sha256(b).hexdigest()
def encoded(v): return (json.dumps(v,ensure_ascii=False,indent=2)+'\n').encode()
def put(path,data,media='application/json',dimensions=None):
    b=data if isinstance(data,bytes) else encoded(data)
    p=OUT/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
    f=dict(path=path,sha256=sha(b),bytes=len(b),media_type=media)
    if dimensions is not None:f['dimensions']=dimensions
    return f
OUT.mkdir(parents=True,exist_ok=True)
source=(SRC/'source.txt').read_text();alignment=json.loads((SRC/'alignment.json').read_text());old_groups=json.loads((SRC/'read-along.json').read_text())['groups']
source_ref=put('source.txt',(SRC/'source.txt').read_bytes(),'text/plain')
align_ref=put('evidence/alignment.json',(SRC/'alignment.json').read_bytes())
SPLIT=660000;OFFSET=96000;GAP=192000
with wave.open(str(SRC/'audio.wav')) as w:
    assert (w.getnchannels(),w.getsampwidth(),w.getframerate())==(1,2,24000)
    count=w.getnframes();raw=w.readframes(count)
# Exact 2:1 sample-domain mapping; zero-order hold is explicitly diagnostic.
mono=array.array('h');mono.frombytes(raw)
if sys.byteorder!='little':mono.byteswap()
stereo=array.array('h',[0])*(OFFSET*2)
for n,s in enumerate(mono):
    if n==SPLIT:stereo.extend(array.array('h',[0])*(GAP*2))
    stereo.extend([s,s,s,s])
last=OFFSET+count*2+GAP;total=last+72000
stereo.extend(array.array('h',[0])*(72000*2))
if sys.byteorder!='little':stereo.byteswap()
(OUT/'audio').mkdir(exist_ok=True)
with wave.open(str(OUT/'audio/master.wav'),'wb') as w:
    w.setparams((2,2,48000,total,'NONE','not compressed'));w.writeframes(stereo.tobytes())
audio=put('audio/master.wav',(OUT/'audio/master.wav').read_bytes(),'audio/wav')
map_doc=dict(schema_version=1,source_audio_sha256=sha(raw if False else (SRC/'audio.wav').read_bytes()),source_sample_rate=24000,delivery_sample_rate=48000,segments=[dict(source=[0,SPLIT],destination=[OFFSET,OFFSET+SPLIT*2]),dict(source=[SPLIT,count],destination=[OFFSET+SPLIT*2+GAP,last])],gaps=[[0,OFFSET],[OFFSET+SPLIT*2,OFFSET+SPLIT*2+GAP],[last,total]])
map_ref=put('presentation-map.json',map_doc)
def mapped(n):return n*2+OFFSET+(GAP if n>=SPLIT else 0)
words=[]
for i,w in enumerate(alignment['words']):
    a,b=w['source_span'];end=alignment['words'][i+1]['source_span'][0] if i+1<len(alignment['words']) else len(source)
    assert not (w['start_sample']<SPLIT<w['end_sample'])
    words.append(dict(id=w['id'],source_span=[a,b],text=source[a:b],following_text=source[b:end],start_sample=mapped(w['start_sample']),end_sample=mapped(w['end_sample'])))
groups=[dict(id=g['id'],word_ids=g['word_ids'],timing_owner_word_id=g['interval_word_id'],start_sample=mapped(g['start_sample']),end_sample=mapped(g['end_sample'])) for g in old_groups]
read=dict(schema_version=1,policy='point-following-same-boundary-v1',language='en',source_sha256=source_ref['sha256'],narration_sha256=map_doc['source_audio_sha256'],alignment_sha256=align_ref['sha256'],presentation_map_sha256=map_ref['sha256'],delivery_audio_sha256=audio['sha256'],leading_text=source[:words[0]['source_span'][0]],words=words,groups=groups,preferred_break_after=[])
read_ref=put('read-along.json',read)
png=(SRC/'n_em.png').read_bytes();dims=list(struct.unpack('>II',png[16:24]));image=put('assets/figure.png',png,'image/png',dims)
reg=json.loads((SRC/'registration.json').read_text())['assets'][0]
card=dict(id='figure-em',image=image,pixel_size=dims,crop_to_source=[1,0,0,1,reg['registration']['crop'][0]-2,reg['registration']['crop'][1]-2],pivot_top_left=[reg['ground_pivot'][0],1-reg['ground_pivot'][1]],stage_size=reg['dimensions'],body_height=reg['dimensions'][1],support_id='deck',member_ids=['em'],allow_mirror=False)
actors=[card];actors_ref=put('actors.json',actors)
def bounds(x,y,z):return dict(x=x,y=y,z=z)
d=3+max(7/(.9*math.tan(math.radians(20))),11/(.9*(16/9)*math.tan(math.radians(20))))
camera=dict(position=[0,4,d],target=[0,4,0],up=[0,1,0],fov_y_degrees=40,near=.1,far=100)
profile=dict(id='paper-player-r186-v1',status='selected_unverified',three_version='0.186.0',aspect_ratio=[16,9],max_dpr=1.5,max_image_dimension=4096,max_graphics_bytes=268435456,max_master_bytes=268435456,outer_bounds=bounds([-11,11],[-3,11],[-5.6,3]),camera=camera,camera_target_offset=[.5,.25],camera_distance_extra=2,pointer_offset=[.8,.4],font_family='Noto Serif',font_css_px=22,line_height=1.3,text_size_range=[1,2],line_count=[2,3],cutout_alpha=.35,shell_alpha=.10,curtain_alpha=.05,shadow_map_size=2048,shadow_type='PCFShadowMap',image_color_space='srgb',working_color_space='linear',tone_mapping='none',texture_filter='linear',mipmaps=False)
profile_ref=put('profile.json',profile)
panel=dict(bounds=bounds([-7,7],[-2.8,-.4],[2.8,2.8]),inset=bounds([-6.5,6.5],[-2.55,-.65],[2.8,2.8]),font=None,text='')
board=dict(bounds=bounds([-4,4],[5,7],[3,3]),inset=bounds([-3.6,3.6],[5.2,6.8],[3,3]),font=None,text='Paper Theater')
shell=dict(schema_version=1,id='diagnostic-shell',style_version='neutral-v1',layers=[dict(id=name,image=None,transform=dict(position=[0,0,z],rotation_xyz=[0,0,0]),dimensions=[22,14],anchors=[]) for name,z in [('inner',1.3),('columns',1.8),('apron',2.4),('plaque',2.8),('signage',3)]],opening=bounds([-8,8],[0,9],[1.3,1.3]),deck=bounds([-8,8],[0,0],[-5.6,2.4]),apron=bounds([-8,8],[-3,0],[2.4,2.4]),supports=[dict(id='deck',position=[0,0,0],normal=[0,1,0])],plaque=panel,title=board,sign=dict(panel={**board,'text':'INTERMISSION'},rope_anchors=[[-3,11,3],[3,11,3]],hidden_clearance=1,lower_ms=1200,raise_ms=800,max_tilt_degrees=3,max_bounce=.15),profile=profile)
shell_ref=put('shell.json',shell)
lighting=dict(ambient=dict(color='#ffffff',intensity=.5),environment=None,lights=[dict(id='key',type='directional',enabled=True,position=[-2,6,5],target=[0,1,0],color='#ffffff',intensity=1,cast_shadow=True,shadow=dict(map_size=2048,bias=0,normal_bias=0,near=.1,far=100))])
# Synthetic image/independent attestations exercise references only; never real approval.
recipe=sha(b'diagnostic-structure-only-v1');attestations=[];reveals=[]
for i,wi in enumerate([2,next(i for i,w in enumerate(words) if w["start_sample"]>=OFFSET+SPLIT*2+GAP)+2]):
    w=words[wi];gid=f'reveal-{i+1}';ids=[]
    for kind in ['image_scope','primary_anchor','independent_anchor']:
        eid=f'{gid}-{kind}';ids.append(eid)
        attestations.append(dict(id=eid,kind=kind,provenance='synthetic' if kind!='primary_anchor' else 'imported',outcome='pass',source_sha256=source_ref['sha256'],narration_sha256=read['narration_sha256'],alignment_sha256=read['alignment_sha256'],presentation_map_sha256=map_ref['sha256'],delivery_audio_sha256=audio['sha256'],recipe_sha256=recipe,target_sha256=image['sha256'] if kind=='image_scope' else audio['sha256'],source_span=w['source_span'],terminal_word_ids=[w['id']],measured_end_sample=None if kind=='image_scope' else w['end_sample'],raw_evidence=align_ref if kind=='primary_anchor' else None))
    reveals.append(dict(id=gid,contributor_ids=[f'em-{i+1}'],not_before_sample=w['end_sample'],facts=[dict(source_span=w['source_span'],image_attestation_id=ids[0],primary_anchor_id=ids[1],independent_anchor_id=ids[2])]))
def snapshot(i,closed=False):return dict(set_id=f'set-{i}',instances=[] if closed else [dict(id=f'em-{i}',asset_id='figure-em',position=[0,0,0],rotation_xyz=[0,0,0],support_id='deck',reveal_group_id=f'reveal-{i}',mirrored=False)],camera=camera,lighting=lighting,curtain_open_ratio=0 if closed else 1)
fade=OFFSET+SPLIT*2
split_word=next(w for w in words if w['start_sample']>=fade+GAP)
scenes=[dict(id='scene-1',set_id='set-1',source_span=[0,split_word['source_span'][0]],narration=[OFFSET,fade],entry_snapshot=snapshot(1),required_asset_ids=['figure-em'],reveal_group_ids=['reveal-1']),dict(id='scene-2',set_id='set-2',source_span=[split_word['source_span'][0],len(source)],narration=[fade+GAP,last],entry_snapshot=snapshot(2),required_asset_ids=['figure-em'],reveal_group_ids=['reveal-2'])]
# Incoming contributor remains ineligible until later speech; this is a diagnostic
# fixture conflict, explicitly missing first-picture/coverage production acceptance.
first=reveals[0]['not_before_sample']
timeline=dict(schema_version=1,capability_version='paper-player-v1',delivery_audio_sha256=audio['sha256'],presentation_map_sha256=map_ref['sha256'],sample_rate=48000,sample_count=total,opening=dict(title='Paper Theater diagnostic',chapter_label='Saved excerpt',title_hold=[0,72000],title_dismiss=[72000,96000],curtain_open=[first,first+72000],first_prose_sample=OFFSET,intro_narration=[OFFSET,first],first_picture_id='scene-1',first_picture_eligible_sample=first),scenes=scenes,transitions=[dict(from_scene_id='scene-1',to_scene_id='scene-2',fade_out_start=fade,swap_sample=fade+48000,fade_in_start=fade+144000,fade_in_end=fade+GAP)],cues=[dict(id='nod-em',scene_id='scene-2',target_id='em-2',action='gesture',start_sample=reveals[1]['not_before_sample'],end_sample=reveals[1]['not_before_sample']+24000,easing='ease_in_out',reveal_group_ids=['reveal-2'],params=dict(gesture='nod',intensity=.25))],closing=dict(last_prose_action_sample=last,curtain_close=[last,total],score_tail=None,terminal_sample=total),terminal_snapshot=snapshot(2,True))
timeline_ref=put('timeline.json',timeline)
stages_ref=put('stages.json',dict(schema_version=1,sets=[dict(id=f'set-{i}',entrances=[],exits=[]) for i in [1,2]]))
bible=put('bible.json',dict(schema_version=1,entities=[dict(id='em',name='Imported diagnostic figure')]))
credits=put('credits.json',dict(provenance='diagnostic',source='The Wonderful Wizard of Oz, L. Frank Baum, Gutenberg 55; saved partial excerpt',media='Imported saved speech/art; original recording identity retained, no Flash audition or new provider calls',transformation='24 kHz mono to 48 kHz stereo by exact sample duplication; new audio identity, synthetic silence gaps, no new mastering acceptance',limitations=['known speech continuity defects','synthetic image and independent-anchor attestations','missing Noto Serif bundle','unverified selected rendering profile','partial chapter; no autonomous acceptance']))
cap=put('captions.vtt',b'WEBVTT\n\nNOTE Diagnostic fixture; measured words in read-along.json\n','text/vtt')
# Neutral poster is a one-pixel diagnostic SVG, not narrative art.
poster=put('poster.svg',b'<svg xmlns="http://www.w3.org/2000/svg" width="16" height="9"><rect width="16" height="9" fill="#222"/></svg>','image/svg+xml')
entries=dict(bible=bible,stages=stages_ref,actors=actors_ref,timeline=timeline_ref,shell=shell_ref,read_along=read_ref,captions=cap,audio=audio,poster=poster,credits=credits)
assets=[image,source_ref,align_ref,map_ref,profile_ref]
inventory=sorted(list(entries.values())+assets,key=lambda f:f['path'])
inventory_bytes=encoded(inventory);put('inventory.json',inventory_bytes)
missing=['autonomous-provenance','complete-chapter','image-scope-review','independent-acoustic-evidence','narrator-continuity','story-coverage','incoming-picture-readiness','rendered-creative-review','font-bundle','profile-r186-browser','target-device-budget','publication-recovery','product-spend-authorization']
acceptance=dict(schema_version=1,provenance='diagnostic',inventory_sha256=sha(inventory_bytes),profile_sha256=profile_ref['sha256'],player_version='pending-r186',checker_version='pt-contracts-v1.0.0',recipe_sha256=recipe,required_check_ids=missing,findings=[dict(id=m,field='acceptance',outcome='missing',message='Pending; diagnostic structure cannot certify this gate',evidence=[]) for m in missing],attestations=attestations)
acceptance_ref=put('acceptance.json',acceptance)
manifest=dict(schema_version=1,capability_version='paper-player-v1',provenance='diagnostic',title='Paper Theater diagnostic',author='L. Frank Baum',chapter_label='Saved partial excerpt',source_sha256=source_ref['sha256'],sample_rate=48000,sample_count=total,entries=entries,assets=assets,reveals=reveals,acceptance=acceptance_ref)
manifest_ref=put('manifest.json',manifest)
put('catalog.json',dict(schema_version=1,releases=[dict(manifest_path='manifest.json',manifest_sha256=manifest_ref['sha256'])]))
doc=dict(manifest=manifest,timeline=timeline,actors=actors,shell=shell,read_along=read,acceptance=acceptance,source_text=source,presentation_map=map_doc)
put('fixture.json',doc)
print(f'fixture: {len(words)} words / {len(groups)} groups; manifest {manifest_ref["sha256"]}; diagnostic only')
