// Explicit schema-v1 mirror of pt-contracts. No generated schemas or wire aliases.
export type Id=string; export type Digest=string; export type Sample=number;
export type Interval=[Sample,Sample]; export type SourceSpan=[number,number];
export type Vec2=[number,number]; export type Vec3=[number,number,number];
export type Provenance='autonomous'|'imported'|'synthetic'|'diagnostic';
export interface FileRef {path:string;sha256:Digest;bytes:number;media_type:string;dimensions?:[number,number]}
export interface ReleaseRef {manifest_path:string;manifest_sha256:Digest}
export type Entries=Record<'bible'|'stages'|'actors'|'timeline'|'shell'|'read_along'|'captions'|'audio'|'poster'|'credits',FileRef>;
export interface Manifest {schema_version:1;capability_version:'paper-player-v1';provenance:Provenance;title:string;author:string;chapter_label:string;source_sha256:Digest;sample_rate:48000;sample_count:Sample;entries:Entries;assets:FileRef[];reveals:RevealGroup[];acceptance:FileRef}
export interface RegisteredCard {id:Id;image:FileRef;pixel_size:[number,number];crop_to_source:[number,number,number,number,number,number];pivot_top_left:Vec2;stage_size:Vec2;body_height:number;support_id:Id;member_ids:Id[];allow_mirror:boolean}
export interface SceneInstance {id:Id;asset_id:Id;position:Vec3;rotation_xyz:Vec3;support_id:Id;reveal_group_id:Id;mirrored:boolean}
export interface RevealFact {source_span:SourceSpan;image_attestation_id:Id;primary_anchor_id:Id;independent_anchor_id:Id}
export interface RevealGroup {id:Id;contributor_ids:Id[];not_before_sample:Sample|null;facts:RevealFact[]}
export interface Scene {id:Id;set_id:Id;source_span:SourceSpan;narration:Interval;entry_snapshot:Snapshot;required_asset_ids:Id[];reveal_group_ids:Id[]}
export interface Camera {position:Vec3;target:Vec3;up:Vec3;fov_y_degrees:number;near:number;far:number}
export interface Shadow {map_size:number;bias:number;normal_bias:number;near:number;far:number}
export type Light={id:Id;enabled:boolean;position:Vec3;target:Vec3;color:string;intensity:number;cast_shadow:boolean;shadow:Shadow}&({type:'directional'}|{type:'spot';angle_radians:number;penumbra:number;distance:number;decay:number});
export interface Lighting {ambient:{color:string;intensity:number};environment:null;lights:Light[]}
export interface Snapshot {set_id:Id;instances:SceneInstance[];camera:Camera;lighting:Lighting;curtain_open_ratio:number}
export interface Opening {title:string;chapter_label:string;title_hold:Interval;title_dismiss:Interval;curtain_open:Interval;first_prose_sample:Sample;intro_narration:Interval|null;first_picture_id:Id;first_picture_eligible_sample:Sample}
export interface Closing {last_prose_action_sample:Sample;curtain_close:Interval;score_tail:Interval|null;terminal_sample:Sample}
export interface SceneTransition {from_scene_id:Id;to_scene_id:Id;fade_out_start:Sample;swap_sample:Sample;fade_in_start:Sample;fade_in_end:Sample}
export type Facing='left'|'right'; export type Easing='linear'|'ease_in'|'ease_out'|'ease_in_out'|'theatrical_wobble';
export type CueAction=
 |{action:'enter';params:{entrance_id:Id;target:Vec3;facing:Facing}}
 |{action:'exit';params:{exit_id:Id}}
 |{action:'move';params:{target:Vec3;facing:Facing|'target';walk_wobble:boolean}}
 |{action:'face';params:{facing:Facing}}
 |{action:'gesture';params:{gesture:'nod'|'shrug'|'recoil'|'bow'|'wobble';intensity:number}}
 |{action:'camera';params:{framing:'stage'|'actor'|'group'|'custom';position:Vec3;target:Vec3;actor_ids:Id[];group_ids:Id[]}}
 |{action:'lighting';params:{lighting:Lighting}};
export type StoryCue={id:Id;scene_id:Id;target_id:Id;start_sample:Sample;end_sample:Sample;easing:Easing;reveal_group_ids:Id[]}&CueAction;
export interface Timeline {schema_version:1;capability_version:'paper-player-v1';delivery_audio_sha256:Digest;presentation_map_sha256:Digest;sample_rate:48000;sample_count:Sample;opening:Opening;scenes:Scene[];transitions:SceneTransition[];cues:StoryCue[];closing:Closing;terminal_snapshot:Snapshot}
export interface ReadAlongWord {id:Id;source_span:SourceSpan;text:string;following_text:string;start_sample:Sample;end_sample:Sample}
export interface ReadAlongGroup {id:Id;word_ids:Id[];timing_owner_word_id:Id;start_sample:Sample;end_sample:Sample}
export interface ReadAlong {schema_version:1;policy:'point-following-same-boundary-v1';language:string;source_sha256:Digest;narration_sha256:Digest;alignment_sha256:Digest;presentation_map_sha256:Digest;delivery_audio_sha256:Digest;leading_text:string;words:ReadAlongWord[];groups:ReadAlongGroup[];preferred_break_after:Id[]}
export interface Bounds {x:Vec2;y:Vec2;z:Vec2}
export interface Anchor {id:Id;position:Vec3;normal:Vec3}
export interface TextPanel {bounds:Bounds;inset:Bounds;font:FileRef|null;text:string}
export interface ShellLayer {id:Id;image:FileRef|null;transform:{position:Vec3;rotation_xyz:Vec3};dimensions:Vec2;anchors:Anchor[]}
export interface RenderingProfile {id:Id;status:'selected_unverified'|'verified';three_version:string;aspect_ratio:Vec2;max_dpr:number;max_image_dimension:number;max_graphics_bytes:number;max_master_bytes:number;outer_bounds:Bounds;camera:Camera;camera_target_offset:Vec2;camera_distance_extra:number;pointer_offset:Vec2;font_family:string;font_css_px:number;line_height:number;text_size_range:Vec2;line_count:[number,number];cutout_alpha:number;shell_alpha:number;curtain_alpha:number;shadow_map_size:number;shadow_type:string;image_color_space:string;working_color_space:string;tone_mapping:string;texture_filter:string;mipmaps:boolean}
export interface Shell {schema_version:1;id:Id;style_version:string;layers:ShellLayer[];opening:Bounds;deck:Bounds;apron:Bounds;supports:Anchor[];plaque:TextPanel;title:TextPanel;sign:{panel:TextPanel;rope_anchors:[Vec3,Vec3];hidden_clearance:number;lower_ms:number;raise_ms:number;max_tilt_degrees:number;max_bounce:number};profile:RenderingProfile}
export type ReviewOutcome='pass'|'fail'|'uncertain'|'missing';
export interface ArtifactRef {id:Id;file:FileRef;provenance:Provenance}
export interface Finding {id:Id;field:string;outcome:ReviewOutcome;message:string;evidence:ArtifactRef[]}
export interface Attestation {id:Id;kind:'image_scope'|'primary_anchor'|'independent_anchor';provenance:Provenance;outcome:ReviewOutcome;source_sha256:Digest;narration_sha256:Digest;alignment_sha256:Digest;presentation_map_sha256:Digest;delivery_audio_sha256:Digest;recipe_sha256:Digest;target_sha256:Digest;source_span:SourceSpan;terminal_word_ids:Id[];measured_end_sample:Sample|null;raw_evidence:FileRef|null}
export interface Acceptance {schema_version:1;provenance:Provenance;inventory_sha256:Digest;profile_sha256:Digest;player_version:string;checker_version:string;recipe_sha256:Digest;required_check_ids:Id[];findings:Finding[];attestations:Attestation[]}
export interface PresentationMap {schema_version:1;source_audio_sha256:Digest;source_sample_rate:number;delivery_sample_rate:48000;segments:{source:Interval;destination:Interval}[];gaps:Interval[]}
export interface PlaybackDocument {manifest:Manifest;timeline:Timeline;actors:RegisteredCard[];shell:Shell;read_along:ReadAlong;acceptance:Acceptance;source_text:string;presentation_map:PresentationMap}
export interface ContractFailure {code:string;field:string;message:string}
