//! All 11 coats, eye colours, catchlight styles, collars and resolved shader uniforms —
//! 1:1 mirror of `web/cat.js` COATS/EYES/SHINES/COLLARS.

use crate::math::hex_color;

#[derive(Clone, Copy, Debug)]
pub struct CoatSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub fur: &'static str,
    pub ink: &'static str,
    pub lid: Option<&'static str>,
    pub rim: &'static str,
    pub arm: &'static str,
    pub paw: &'static str,
    pub toe: &'static str,
    pub whisker: &'static str,
    pub eye: &'static str,
    pub light: Option<&'static str>,
    pub muzzle: f32,
    pub bib: f32,
    pub tuxedo: f32,
    pub nose: Option<&'static str>,
    pub mouth: Option<&'static str>,
    pub inner: Option<&'static str>,
    pub inner_amt: f32,
    pub shade: f32,
    pub tabby: Option<&'static str>,
    pub striped: bool,
    pub patch_a: Option<&'static str>,
    pub patch_b: Option<&'static str>,
    pub calico: f32,
    pub point: Option<&'static str>,
    pub points: f32,
    pub wave: Option<&'static str>,
    pub curl: f32,
    pub ears: f32,
}

const DEFAULT_COAT: CoatSpec = CoatSpec {
    id: "black", label: "黑猫",
    fur: "#151515", ink: "#151515", lid: None, rim: "#f6f3e8", arm: "#8a8984",
    paw: "#f7f6f0", toe: "#b9b7ae", whisker: "#dedcd4", eye: "gold",
    light: None, muzzle: 0.0, bib: 0.0, tuxedo: 0.0, nose: None, mouth: None,
    inner: None, inner_amt: 0.0, shade: 0.0, tabby: None, striped: false,
    patch_a: None, patch_b: None, calico: 0.0, point: None, points: 0.0,
    wave: None, curl: 0.0, ears: 1.0,
};

pub const COATS: &[CoatSpec] = &[
    DEFAULT_COAT,
    CoatSpec {
        id: "tuxedo", label: "奶牛猫",
        fur: "#1a1a1a", ink: "#151515", lid: Some("#e4c25a"), rim: "#f6f3e8", arm: "#8a8984",
        paw: "#f8f7f2", toe: "#bcbab2", whisker: "#e8e6de", eye: "green",
        light: Some("#f8f7f2"), muzzle: 1.0, tuxedo: 1.0, nose: Some("#e7a0a0"),
        mouth: Some("#9a8f88"), inner: Some("#3a2e2e"), inner_amt: 1.0, shade: 0.25,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "orange", label: "橘猫",
        fur: "#eda25a", ink: "#b8712f", lid: Some("#8a4a1f"), rim: "#fff8ea", arm: "#b8712f",
        paw: "#fff6e8", toe: "#e0b991", whisker: "#fffaf0", eye: "green",
        light: Some("#fff1dc"), muzzle: 1.0, bib: 1.0, nose: Some("#e58a78"),
        mouth: Some("#a5643a"), inner: Some("#f4b6a2"), inner_amt: 1.0, shade: 0.55,
        tabby: Some("#c46a2b"), striped: true,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "grey", label: "蓝灰猫",
        fur: "#8995a4", ink: "#5f6a78", lid: Some("#3d4450"), rim: "#f6f5f0", arm: "#5f6a78",
        paw: "#f2f1ec", toe: "#bfc3c8", whisker: "#f4f2ea", eye: "amber",
        light: Some("#b8c1cc"), muzzle: 0.8, bib: 0.6, nose: Some("#6d6f7d"),
        mouth: Some("#56606d"), inner: Some("#c9aeb4"), inner_amt: 1.0, shade: 0.55,
        tabby: Some("#5e6a79"), striped: false,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "white", label: "白猫",
        fur: "#fbf9f3", ink: "#c2b8a8", lid: Some("#8a7f72"), rim: "#8b8172", arm: "#c9c1b2",
        paw: "#fbf9f3", toe: "#e0c9c2", whisker: "#b6ada0", eye: "blue",
        nose: Some("#f0a3a3"), mouth: Some("#c99c94"), inner: Some("#f6c3c3"),
        inner_amt: 1.0, shade: 0.5,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "calico", label: "三花猫",
        fur: "#fbf8f1", ink: "#c2b8a8", lid: Some("#6e5a4e"), rim: "#8b8172", arm: "#c9c1b2",
        paw: "#fbf8f1", toe: "#e0c9c2", whisker: "#b6ada0", eye: "gold",
        patch_a: Some("#e5964a"), patch_b: Some("#2b2522"), calico: 1.0,
        nose: Some("#eea29a"), mouth: Some("#b08a80"), inner: Some("#f2bcb6"),
        inner_amt: 1.0, shade: 0.5,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "siamese", label: "暹罗猫",
        fur: "#f0e4cf", ink: "#8b6f5c", rim: "#8b7a68", arm: "#6e5444",
        paw: "#5a4033", toe: "#3b2a21", whisker: "#f3ece0", eye: "blue",
        point: Some("#4b3428"), points: 1.0, nose: Some("#2f211b"),
        mouth: Some("#24180f"), inner: Some("#6d5244"), inner_amt: 1.0, shade: 0.45,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "ragdoll", label: "布偶猫",
        fur: "#f5eee3", ink: "#a8927f", rim: "#8b8172", arm: "#a8927f",
        paw: "#fdfbf7", toe: "#ddd2c6", whisker: "#cfc4b6", eye: "blue",
        point: Some("#6f5242"), points: 1.0, light: Some("#fdfbf7"),
        muzzle: 1.0, tuxedo: 1.0, bib: 1.0, nose: Some("#e39b98"),
        mouth: Some("#a07e70"), inner: Some("#9a7466"), inner_amt: 1.0, shade: 0.5,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "devon", label: "德文卷毛猫",
        fur: "#b3a8ad", ink: "#7f7479", lid: Some("#5b5156"), rim: "#fbf9f6", arm: "#7f7479",
        paw: "#efeaec", toe: "#c3b9be", whisker: "#fbf9f6", eye: "gold",
        light: Some("#d9d1d5"), muzzle: 0.8, wave: Some("#9a8f95"), curl: 1.0,
        ears: 1.45, nose: Some("#d79a9e"), mouth: Some("#7f7479"), inner: Some("#e8b9bf"),
        inner_amt: 1.0, shade: 0.5,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "silver", label: "美短银虎斑",
        fur: "#c7ccd1", ink: "#7d838a", lid: Some("#3f444a"), rim: "#fbfbf8", arm: "#7d838a",
        paw: "#f1f2f2", toe: "#b6bcc2", whisker: "#fbfbf8", eye: "green",
        light: Some("#f1f2f2"), muzzle: 1.0, bib: 0.8, nose: Some("#c98a8a"),
        mouth: Some("#6c7279"), inner: Some("#e5b8bb"), inner_amt: 1.0, shade: 0.55,
        tabby: Some("#2e3136"), striped: true,
        ..DEFAULT_COAT
    },
    CoatSpec {
        id: "lihua", label: "狸花猫",
        fur: "#a98a62", ink: "#6e5640", lid: Some("#3d2f22"), rim: "#fff8ec", arm: "#6e5640",
        paw: "#efe5d3", toe: "#c9b493", whisker: "#fff8ec", eye: "green",
        light: Some("#eadfca"), muzzle: 1.0, bib: 0.7, nose: Some("#b8735f"),
        mouth: Some("#5e4836"), inner: Some("#d9a894"), inner_amt: 1.0, shade: 0.55,
        tabby: Some("#3d2f22"), striped: true,
        ..DEFAULT_COAT
    },
];

pub fn find_coat(id: &str) -> &'static CoatSpec {
    COATS.iter().find(|c| c.id == id).unwrap_or(&COATS[0])
}

pub fn eye_hex(id: &str) -> &'static str {
    match id {
        "green" => "#93c255",
        "blue" => "#6eb2e6",
        "amber" => "#e59a32",
        "copper" => "#d9792f",
        _ => "#dcb64a",
    }
}

pub fn collar_hex(id: &str) -> Option<&'static str> {
    match id {
        "red" => Some("#d9483b"),
        "blue" => Some("#4f8fd8"),
        "pink" => Some("#ee8fb4"),
        "green" => Some("#5fae6a"),
        _ => None,
    }
}

/// Catchlight dots per style: `[x (outer side +), y, radius]`.
pub fn shine_dots(style: &str) -> &'static [[f32; 3]] {
    match style {
        "none" => &[],
        "sparkle" => &[
            [0.035, 0.045, 0.042],
            [-0.04, -0.045, 0.02],
            [0.06, -0.03, 0.011],
        ],
        _ => &[[0.04, 0.05, 0.03], [-0.035, -0.045, 0.014]],
    }
}

#[derive(Clone, Debug)]
pub struct Look {
    pub style_3d: bool,
    pub coat: String,
    pub pattern: String,
    pub eye: String,
    pub shine: String,
    pub blush: bool,
    pub whiskers: bool,
    pub collar: String,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            style_3d: false,
            coat: "black".into(),
            pattern: "auto".into(),
            eye: "auto".into(),
            shine: "classic".into(),
            blush: false,
            whiskers: true,
            collar: "none".into(),
        }
    }
}

/// Resolved RGB colours & marking weights ready for GPU constant buffers.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedLook {
    pub fur: [f32; 3],
    pub light: [f32; 3],
    pub dark: [f32; 3],
    pub patch_a: [f32; 3],
    pub patch_b: [f32; 3],
    pub point: [f32; 3],
    pub inner: [f32; 3],
    pub stripes: f32,
    pub muzzle: f32,
    pub bib: f32,
    pub calico: f32,
    pub tuxedo: f32,
    pub points: f32,
    pub curl: f32,
    pub inner_amt: f32,
    pub shade: f32,
    pub chin_visible: bool,
    pub chin_color: [f32; 3],
    pub rim_color: [f32; 3],
    pub arm_rim_color: [f32; 3],
    pub paw_color: [f32; 3],
    pub toe_color: [f32; 3],
    pub whisker_color: [f32; 3],
    pub nose_color: [f32; 3],
    pub mouth_color: [f32; 3],
    pub face_visible: bool,
    pub eye_colors: [[f32; 3]; 2],
    pub lid_color: [f32; 3],
    pub ear_scale: f32,
    pub collar_visible: bool,
    pub collar_color: [f32; 3],
    pub whiskers_visible: bool,
    pub blush_always: bool,
    pub style_3d: bool,
}

impl Look {
    pub fn resolve(&self) -> ResolvedLook {
        let c = find_coat(&self.coat);
        let striped = c.tabby.is_some()
            && (self.pattern == "tabby" || (self.pattern == "auto" && c.striped));
        let fur = hex_color(c.fur);
        let dark_hex = c.tabby.or(c.wave).unwrap_or(c.fur);
        let eye_key = if self.eye == "auto" { c.eye } else { self.eye.as_str() };
        let eye_left = hex_color(if eye_key == "odd" { eye_hex("blue") } else { eye_hex(eye_key) });
        let eye_right = hex_color(if eye_key == "odd" { eye_hex("gold") } else { eye_hex(eye_key) });
        let lid_color = c.lid.map(hex_color).unwrap_or(eye_left);
        let collar_col = collar_hex(&self.collar);
        ResolvedLook {
            fur,
            light: c.light.map(hex_color).unwrap_or(fur),
            dark: hex_color(dark_hex),
            patch_a: c.patch_a.map(hex_color).unwrap_or(fur),
            patch_b: c.patch_b.map(hex_color).unwrap_or(fur),
            point: c.point.map(hex_color).unwrap_or(fur),
            inner: c.inner.map(hex_color).unwrap_or(fur),
            stripes: if striped { 1.0 } else { 0.0 },
            muzzle: c.muzzle,
            bib: c.bib,
            calico: c.calico,
            tuxedo: c.tuxedo,
            points: c.points,
            curl: c.curl,
            inner_amt: c.inner_amt,
            shade: if self.style_3d { c.shade } else { 0.0 },
            chin_visible: !self.style_3d,
            chin_color: hex_color(c.ink),
            rim_color: hex_color(c.rim),
            arm_rim_color: hex_color(c.arm),
            paw_color: hex_color(c.paw),
            toe_color: hex_color(c.toe),
            whisker_color: hex_color(c.whisker),
            nose_color: c.nose.map(hex_color).unwrap_or(fur),
            mouth_color: c.mouth.map(hex_color).unwrap_or(fur),
            face_visible: c.nose.is_some(),
            eye_colors: [eye_left, eye_right],
            lid_color,
            ear_scale: c.ears,
            collar_visible: collar_col.is_some(),
            collar_color: collar_col.map(hex_color).unwrap_or(hex_color("#d9483b")),
            whiskers_visible: self.whiskers,
            blush_always: self.blush,
            style_3d: self.style_3d,
        }
    }
}
