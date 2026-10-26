"""Pure native-aspect registration from measured source pixels and contacts."""
import math


def register_card(registration,*,body_pixels,body_height,mask_status='measured'):
    x,y,w,h=registration['crop'];padding=registration['padding']
    bx,by,bw,bh=body_pixels
    ow,oh=registration['original_size']
    values=[x,y,w,h,padding,bx,by,bw,bh,body_height,ow,oh]
    if not all(type(v) in [int,float] and math.isfinite(v) for v in values):raise ValueError('nonfinite geometry')
    if mask_status!='measured' or not (0<ow<=4096 and 0<oh<=4096 and 0<=padding<=16):raise ValueError('unaccepted extraction or dimensions')
    if not (0<=x<x+w<=ow and 0<=y<y+h<=oh and x<=bx<bx+bw<=x+w and y<=by<by+bh<=y+h and body_height>0):raise ValueError('body/crop outside source')
    contacts=registration['contacts']
    if not contacts:raise ValueError('missing measured support contacts')
    for contact in contacts:
        if (len(contact)!=2 or not all(math.isfinite(n) for n in contact)
                or not x<=contact[0]<=x+w or not y<=contact[1]<=y+h
                or contact not in registration['measured_supports']):raise ValueError('unmeasured or out-of-crop contact')
    scale=body_height/bh
    width=w+2*padding;height=h+2*padding
    contacts=registration['contacts']
    cx=sum(p[0] for p in contacts)/len(contacts);cy=sum(p[1] for p in contacts)/len(contacts)
    return {'stage_size':[width*scale,height*scale],'body_height':body_height,
            'pivot_top_left':[(cx-x+padding)/width,(cy-y+padding)/height],
            'crop_to_source':[1,0,0,1,x-padding,y-padding],
            'production_approved':False}
