use crate::*;
use serde::{Deserialize, Serialize};
pub type Vec3 = [f64; 3];
pub type Vec2 = [f64; 2];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub capability_version: String,
    pub provenance: Provenance,
    pub title: String,
    pub author: String,
    pub chapter_label: String,
    pub source_sha256: Digest,
    pub sample_rate: u32,
    pub sample_count: Sample,
    pub entries: Entries,
    pub assets: Vec<FileRef>,
    pub reveals: Vec<RevealGroup>,
    pub acceptance: FileRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entries {
    pub bible: FileRef,
    pub stages: FileRef,
    pub actors: FileRef,
    pub timeline: FileRef,
    pub shell: FileRef,
    pub read_along: FileRef,
    pub captions: FileRef,
    pub audio: FileRef,
    pub poster: FileRef,
    pub credits: FileRef,
}
impl Entries {
    pub fn files(&self) -> [&FileRef; 10] {
        [
            &self.bible,
            &self.stages,
            &self.actors,
            &self.timeline,
            &self.shell,
            &self.read_along,
            &self.captions,
            &self.audio,
            &self.poster,
            &self.credits,
        ]
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredCard {
    pub id: Id,
    pub image: FileRef,
    pub pixel_size: [u32; 2],
    pub crop_to_source: [f64; 6],
    pub pivot_top_left: Vec2,
    pub stage_size: Vec2,
    pub body_height: f64,
    pub support_id: Id,
    pub member_ids: Vec<Id>,
    pub allow_mirror: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneInstance {
    pub id: Id,
    pub asset_id: Id,
    pub position: Vec3,
    pub rotation_xyz: Vec3,
    pub support_id: Id,
    pub reveal_group_id: Id,
    pub mirrored: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealFact {
    pub source_span: SourceSpan,
    pub image_attestation_id: Id,
    pub primary_anchor_id: Id,
    pub independent_anchor_id: Id,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealGroup {
    pub id: Id,
    pub contributor_ids: Vec<Id>,
    pub not_before_sample: Option<Sample>,
    pub facts: Vec<RevealFact>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: Id,
    pub set_id: Id,
    pub source_span: SourceSpan,
    pub narration: Interval,
    pub entry_snapshot: Snapshot,
    pub required_asset_ids: Vec<Id>,
    pub reveal_group_ids: Vec<Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub set_id: Id,
    pub instances: Vec<SceneInstance>,
    pub camera: Camera,
    pub lighting: Lighting,
    pub curtain_open_ratio: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_degrees: f64,
    pub near: f64,
    pub far: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ambient {
    pub color: String,
    pub intensity: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    pub map_size: u32,
    pub bias: f64,
    pub normal_bias: f64,
    pub near: f64,
    pub far: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light {
    pub id: Id,
    #[serde(flatten)]
    pub source: LightSource,
    pub enabled: bool,
    pub position: Vec3,
    pub target: Vec3,
    pub color: String,
    pub intensity: f64,
    pub cast_shadow: bool,
    pub shadow: Shadow,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LightSource {
    Directional,
    Spot {
        angle_radians: f64,
        penumbra: f64,
        distance: f64,
        decay: f64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub ambient: Ambient,
    pub environment: (),
    pub lights: Vec<Light>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    pub schema_version: u32,
    pub capability_version: String,
    pub delivery_audio_sha256: Digest,
    pub presentation_map_sha256: Digest,
    pub sample_rate: u32,
    pub sample_count: Sample,
    pub opening: Opening,
    pub scenes: Vec<Scene>,
    pub transitions: Vec<SceneTransition>,
    pub cues: Vec<StoryCue>,
    pub closing: Closing,
    pub terminal_snapshot: Snapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opening {
    pub title: String,
    pub chapter_label: String,
    pub title_hold: Interval,
    pub title_dismiss: Interval,
    pub curtain_open: Interval,
    pub first_prose_sample: Sample,
    pub intro_narration: Option<Interval>,
    pub first_picture_id: Id,
    pub first_picture_eligible_sample: Sample,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closing {
    pub last_prose_action_sample: Sample,
    pub curtain_close: Interval,
    pub score_tail: Option<Interval>,
    pub terminal_sample: Sample,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTransition {
    pub from_scene_id: Id,
    pub to_scene_id: Id,
    pub fade_out_start: Sample,
    pub swap_sample: Sample,
    pub fade_in_start: Sample,
    pub fade_in_end: Sample,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveFacing {
    Left,
    Right,
    Target,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    Nod,
    Shrug,
    Recoil,
    Bow,
    Wobble,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Framing {
    Stage,
    Actor,
    Group,
    Custom,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    TheatricalWobble,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CueAction {
    Enter {
        entrance_id: Id,
        target: Vec3,
        facing: Facing,
    },
    Exit {
        exit_id: Id,
    },
    Move {
        target: Vec3,
        facing: MoveFacing,
        walk_wobble: bool,
    },
    Face {
        facing: Facing,
    },
    Gesture {
        gesture: Gesture,
        intensity: f64,
    },
    Camera {
        framing: Framing,
        position: Vec3,
        target: Vec3,
        actor_ids: Vec<Id>,
        group_ids: Vec<Id>,
    },
    Lighting {
        lighting: Lighting,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryCue {
    pub id: Id,
    pub scene_id: Id,
    pub target_id: Id,
    pub start_sample: Sample,
    pub end_sample: Sample,
    pub easing: Easing,
    pub reveal_group_ids: Vec<Id>,
    #[serde(flatten)]
    pub action: CueAction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAlong {
    pub schema_version: u32,
    pub policy: String,
    pub language: String,
    pub source_sha256: Digest,
    pub narration_sha256: Digest,
    pub alignment_sha256: Digest,
    pub presentation_map_sha256: Digest,
    pub delivery_audio_sha256: Digest,
    pub leading_text: String,
    pub words: Vec<ReadAlongWord>,
    pub groups: Vec<ReadAlongGroup>,
    pub preferred_break_after: Vec<Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAlongWord {
    pub id: Id,
    pub source_span: SourceSpan,
    pub text: String,
    pub following_text: String,
    pub start_sample: Sample,
    pub end_sample: Sample,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAlongGroup {
    pub id: Id,
    pub word_ids: Vec<Id>,
    pub timing_owner_word_id: Id,
    pub start_sample: Sample,
    pub end_sample: Sample,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub position: Vec3,
    pub rotation_xyz: Vec3,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub id: Id,
    pub position: Vec3,
    pub normal: Vec3,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellLayer {
    pub id: Id,
    pub image: Option<FileRef>,
    pub transform: Transform,
    pub dimensions: Vec2,
    pub anchors: Vec<Anchor>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub x: Vec2,
    pub y: Vec2,
    pub z: Vec2,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextPanel {
    pub bounds: Bounds,
    pub inset: Bounds,
    pub font: Option<FileRef>,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sign {
    pub panel: TextPanel,
    pub rope_anchors: [Vec3; 2],
    pub hidden_clearance: f64,
    pub lower_ms: u32,
    pub raise_ms: u32,
    pub max_tilt_degrees: f64,
    pub max_bounce: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderingProfile {
    pub id: Id,
    pub status: ProfileStatus,
    pub three_version: String,
    pub aspect_ratio: Vec2,
    pub max_dpr: f64,
    pub max_image_dimension: u32,
    pub max_graphics_bytes: u64,
    pub max_master_bytes: u64,
    pub outer_bounds: Bounds,
    pub camera: Camera,
    pub camera_target_offset: Vec2,
    pub camera_distance_extra: f64,
    pub pointer_offset: Vec2,
    pub font_family: String,
    pub font_css_px: f64,
    pub line_height: f64,
    pub text_size_range: Vec2,
    pub line_count: [u32; 2],
    pub cutout_alpha: f64,
    pub shell_alpha: f64,
    pub curtain_alpha: f64,
    pub shadow_map_size: u32,
    pub shadow_type: String,
    pub image_color_space: String,
    pub working_color_space: String,
    pub tone_mapping: String,
    pub texture_filter: String,
    pub mipmaps: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    SelectedUnverified,
    Verified,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shell {
    pub schema_version: u32,
    pub id: Id,
    pub style_version: String,
    pub layers: Vec<ShellLayer>,
    pub opening: Bounds,
    pub deck: Bounds,
    pub apron: Bounds,
    pub supports: Vec<Anchor>,
    pub plaque: TextPanel,
    pub title: TextPanel,
    pub sign: Sign,
    pub profile: RenderingProfile,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    ImageScope,
    PrimaryAnchor,
    IndependentAnchor,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attestation {
    pub id: Id,
    pub kind: EvidenceKind,
    pub provenance: Provenance,
    pub outcome: ReviewOutcome,
    pub source_sha256: Digest,
    pub narration_sha256: Digest,
    pub alignment_sha256: Digest,
    pub presentation_map_sha256: Digest,
    pub delivery_audio_sha256: Digest,
    pub recipe_sha256: Digest,
    pub target_sha256: Digest,
    pub source_span: SourceSpan,
    pub terminal_word_ids: Vec<Id>,
    pub measured_end_sample: Option<Sample>,
    pub raw_evidence: Option<FileRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub schema_version: u32,
    pub provenance: Provenance,
    pub inventory_sha256: Digest,
    pub profile_sha256: Digest,
    pub player_version: String,
    pub checker_version: String,
    pub recipe_sha256: Digest,
    pub required_check_ids: Vec<Id>,
    pub findings: Vec<Finding>,
    pub attestations: Vec<Attestation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSegment {
    pub source: Interval,
    pub destination: Interval,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationMap {
    pub schema_version: u32,
    pub source_audio_sha256: Digest,
    pub source_sample_rate: u32,
    pub delivery_sample_rate: u32,
    pub segments: Vec<PresentationSegment>,
    pub gaps: Vec<Interval>,
}
/// Portable test/load envelope; each payload still has its own exact-byte FileRef.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybackDocument {
    pub manifest: Manifest,
    pub timeline: Timeline,
    pub actors: Vec<RegisteredCard>,
    pub shell: Shell,
    pub read_along: ReadAlong,
    pub acceptance: Acceptance,
    pub source_text: String,
    pub presentation_map: PresentationMap,
}
