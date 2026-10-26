"""Cheap diagnostic schedule and ownership checks before dependent artwork."""
SUPPORTED={'enter','exit','move','face','nod','shrug','recoil','bow','wobble','camera','lighting'}

def validate_plan(plan):
    findings=[]
    def fail(id,field):
        item={'id':id,'field':field,'outcome':'fail'}
        if item not in findings:findings.append(item)
    if plan['source_sha256']!=plan['observed_source_sha256']:fail('stale-source','source')
    pictures={p['id']:p for p in plan['pictures']};actions={a['id']:a for a in plan['actions']}
    for beat in plan['beats']:
        visible={i['subject_id'] for pid in beat['picture_ids'] if pid in pictures for i in pictures[pid]['instances']}
        if not set(beat['subjects'])<=visible:fail('missing-subject',beat['id'])
        for id in beat['required_actions']:
            if id not in actions:fail('omitted-action',id)
    for picture in pictures.values():
        id=picture['id'];a,b=picture['interval']
        if not 0<=a<b<=plan['sample_count']:fail('picture-window',id)
        owners={}
        for item in picture['instances']:owners.setdefault(item['subject_id'],set()).add(item['owner_id'])
        if not set(picture['required_subjects'])<=set(owners):fail('empty-entrance',id)
        for subject in picture['required_subjects']:
            if picture['ready'].get(subject) is None or picture['ready'][subject]>a:fail('unready-entrance',id)
        if any(len(values)!=1 for values in owners.values()):fail('duplicate-owner',id)
    for reference in plan['references']:
        if reference['permitted_source_end']>reference['asset_source_end']:fail('later-reference',reference['id'])
        if reference['environment_empty'] and reference.get('subject_id'):fail('unrelated-empty-reference',reference['id'])
    for action in actions.values():
        id=action['id'];picture=pictures.get(action['picture_id']);a,b=action['interval']
        if action['kind'] not in SUPPORTED:fail('unsupported-action',id)
        if picture is None or not picture['interval'][0]<=a<b<=picture['interval'][1]:fail('impossible-action',id)
        elif picture['ready'].get(action['subject_id']) is None or picture['ready'][action['subject_id']]>a:fail('impossible-action',id)
    review=plan['review']
    if not set(pictures)<=set(review['opening_picture_ids']):fail('missing-opening-review','review')
    if not set(actions)<=set(review['action_ids']) or not set(actions)<=set(review['paired_motion_ids']):fail('missing-action-review','review')
    if any(review['hold_samples'].get(id,0)<p['interval'][1]-p['interval'][0] for id,p in pictures.items()):fail('missing-hold-review','review')
    if not {'source','style','reference','extraction','stage'}<=set(review['image_scopes']):fail('missing-image-scope','review')
    end=0
    for a,b in sorted(review['audio_windows']):
        if not 0<=a<=end<b<=plan['sample_count']:fail('incomplete-audio-review','review')
        end=max(end,b)
    if end!=plan['sample_count']:fail('incomplete-audio-review','review')
    if review['semantic_outcome']!='pass':fail('uncertain-semantic-review','review')
    return {'findings':findings,'preflight_ready':not findings,'production_approved':False}
