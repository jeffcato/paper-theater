"""Cached Gutenberg chapter selection with Unicode scalar provenance."""
import hashlib
import re


def digest(value):
    return hashlib.sha256(value).hexdigest()


def clean_source(raw):
    text=raw.decode('utf-8')
    start=re.search(r'(?im)^\*\*\* START OF (?:THE|THIS) PROJECT GUTENBERG[^\n]*\n',text)
    end=re.search(r'(?im)^\*\*\* END OF (?:THE|THIS) PROJECT GUTENBERG',text)
    lo=start.end() if start else 0
    hi=end.start() if end else len(text)
    # Blank editorial illustrations, preserving raw scalar coordinates.
    body=re.sub(r'(?is)\[(?:Illustration|Transcriber.s Note|Footnote|Note)[^\]]*\]',
                lambda m: ' '*len(m[0]),text[lo:hi])
    body=re.sub(r'(?<!\w)_([^_]+)_(?!\w)',lambda m:' '+m[1]+' ',body)
    paragraphs=[]; current=[]
    def flush():
        if current: paragraphs.append(current[:]);current.clear()
    for line in re.finditer(r'[^\n]+|\n',body):
        if line[0]=='\n': continue
        words=list(re.finditer(r'\S+',line[0]))
        # Actual blank lines also occur as whitespace-only lines.
        if not words: flush();continue
        previous=body[max(0,line.start()-2):line.start()]
        if previous.endswith('\n\n'):flush()
        value=line[0].strip()
        heading=bool(re.fullmatch(r'(?:Chapter\s+[IVXLCDM\d]+(?:\..*)?|[IVXLCDM]+\.|BOOK\s+\w+)',value))
        if heading:flush()
        for w in words:
            a=lo+line.start()+w.start();b=lo+line.start()+w.end()
            if current:current.append((' ',(current[-1][1][1],a)))
            current.extend((text[i],(i,i+1)) for i in range(a,b))
        if heading:flush()
    flush()
    result=[]
    for p in paragraphs:
        if result:
            gap=(result[-1][1][1],p[0][1][0]);result.extend([('\n',gap),('\n',gap)])
        result.extend(p)
    return ''.join(c for c,_ in result),[list(s) for _,s in result]


def freeze_chapter(raw,selector):
    clean,mapping=clean_source(raw)
    paragraphs=list(re.finditer(r'[^\n]+',clean))
    candidates=[]
    for i,p in enumerate(paragraphs):
        if p[0]!=selector['heading']:continue
        if selector.get('parent'):
            parents=[v[0] for v in paragraphs[:i] if re.fullmatch(r'BOOK\s+\w+',v[0])]
            if not parents or parents[-1]!=selector['parent']:continue
        following=next((q for q in paragraphs[i+1:] if q[0]==selector['next_heading']),None)
        if following is None:continue
        a=p.end();b=following.start()
        while a<b and clean[a].isspace():a+=1
        if selector.get('body_title'):
            title=selector['body_title']
            if not clean[a:].startswith(title+'\n\n'):continue
            a+=len(title)+2
        while b>a and clean[b-1].isspace():b-=1
        text=clean[a:b]
        if text.startswith(selector['opening']) and text.endswith(selector['ending']):candidates.append((a,b,text))
    if len(candidates)!=1:raise ValueError('chapter body missing or ambiguous')
    a,b,text=candidates[0]
    return {'schema_version':1,'book_id':selector['book_id'],'selector':selector,
            'raw_sha256':digest(raw),'clean_sha256':digest(clean.encode()),
            'source_sha256':digest(text.encode()),'clean_span':[a,b],
            'raw_span':[mapping[a][0],mapping[b-1][1]],'text':text,
            'scalar_to_raw':mapping[a:b],'provenance':'imported'}


def verify_frozen(raw, frozen):
    if freeze_chapter(raw,frozen['selector'])!=frozen:
        raise ValueError('frozen source identity or scalar mapping changed')
