import json,unittest
from pathlib import Path

class GeometryTests(unittest.TestCase):
    def test_cached_contacts_retain_native_aspect_and_semantic_body_scale(self):
        from geometry import register_card
        root=Path(__file__).resolve().parents[2]/'fixtures/generation/geometry/registration.json'
        asset=json.loads(root.read_text())['assets'][0]
        r=asset['registration']; crop=r['crop'];x,y,w,h=crop
        result=register_card(r,body_pixels=[x,y,w,h],body_height=1.6)
        scale=1.6/h
        self.assertAlmostEqual(result['stage_size'][0],(w+2*r['padding'])*scale)
        self.assertAlmostEqual(result['stage_size'][1],(h+2*r['padding'])*scale)
        self.assertAlmostEqual(result['pivot_top_left'][0],(r['contacts'][0][0]-x+r['padding'])/(w+2*r['padding']))
        self.assertAlmostEqual(result['body_height'],1.6)

    def test_diagnostic_mattes_and_canvas_based_dimensions_are_rejected(self):
        from geometry import register_card
        import copy
        base={'original_size':[1600,1600],'crop':[500,200,400,1000],
              'padding':2,'contacts':[[700,1190]],'measured_supports':[[700,1190]]}
        result=register_card(base,body_pixels=[550,250,300,900],body_height=1.8)
        self.assertAlmostEqual(result['stage_size'][1],1004*.002)
        for defect in ['matte','canvas','nonfinite','missing_contact','unmeasured_contact','crop_bounds']:
            with self.subTest(defect=defect):
                r=copy.deepcopy(base);body=[550,250,300,900];height=1.8;status='measured'
                if defect=='matte':status='diagnostic_white_matte'
                if defect=='canvas':body=[0,0,1600,1600]
                if defect=='nonfinite':height=float('nan')
                if defect=='missing_contact':r['contacts']=[]
                if defect=='unmeasured_contact':r['contacts']=[[700,1180]]
                if defect=='crop_bounds':r['crop']=[1500,200,400,1000]
                with self.assertRaises(ValueError):register_card(r,body_pixels=body,body_height=height,mask_status=status)
