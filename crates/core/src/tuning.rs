//! Behaviour tuning parameters — 1:1 mirror of `web/tuning.js`.

use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Toggle,
    Range,
}

#[derive(Clone, Copy, Debug)]
pub struct ParamMeta {
    pub group: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub def: f32,
    pub unit: &'static str,
    pub hint: &'static str,
}

pub const GROUPS: [&str; 2] = [
    "不耐烦（尾巴捶桌）",
    "好奇（快速移动鼠标）",
];

pub const TUNING_SCHEMA: &[ParamMeta] = &[
    ParamMeta {
        group: GROUPS[0],
        key: "annoyOn",
        label: "会不耐烦",
        kind: ParamKind::Toggle,
        min: 0.0, max: 1.0, step: 1.0, def: 1.0, unit: "",
        hint: "关掉后不会再生气，也不会甩尾巴捶桌。",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "typeRate",
        label: "打字多快会烦",
        kind: ParamKind::Range,
        min: 4.0, max: 20.0, step: 1.0, def: 9.0, unit: "键/秒",
        hint: "持续快于这个速度，烦躁会慢慢累积。",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "mashKeys",
        label: "几个键一起按算乱拍",
        kind: ParamKind::Range,
        min: 3.0, max: 8.0, step: 1.0, def: 4.0, unit: "个",
        hint: "0.15 秒内按下这么多键，算一次乱拍，马上烦一大截。",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "petLimit",
        label: "摸头多久会烦",
        kind: ParamKind::Range,
        min: 2.0, max: 30.0, step: 1.0, def: 4.0, unit: "秒",
        hint: "",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "wakeAnnoy",
        label: "被吵醒有多烦",
        kind: ParamKind::Range,
        min: 0.0, max: 100.0, step: 5.0, def: 35.0, unit: "%",
        hint: "睡着时被键盘或鼠标吵醒，烦躁一下子加这么多。",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "crossAt",
        label: "烦到多少开始捶桌",
        kind: ParamKind::Range,
        min: 20.0, max: 100.0, step: 5.0, def: 60.0, unit: "%",
        hint: "",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "calmRate",
        label: "消气速度",
        kind: ParamKind::Range,
        min: 2.0, max: 50.0, step: 1.0, def: 10.0, unit: "%/秒",
        hint: "安静 2 秒后，每秒消掉这么多烦躁。",
    },
    ParamMeta {
        group: GROUPS[0],
        key: "thumpRate",
        label: "捶桌节奏",
        kind: ParamKind::Range,
        min: 1.0, max: 5.0, step: 0.1, def: 2.6, unit: "下/秒",
        hint: "",
    },
    ParamMeta {
        group: GROUPS[1],
        key: "curiousOn",
        label: "会好奇",
        kind: ParamKind::Toggle,
        min: 0.0, max: 1.0, step: 1.0, def: 1.0, unit: "",
        hint: "快速移动鼠标时，猫瞪大眼睛、瞳孔放大，竖起耳朵盯着光标。",
    },
    ParamMeta {
        group: GROUPS[1],
        key: "curiousSpeed",
        label: "鼠标多快会好奇",
        kind: ParamKind::Range,
        min: 300.0, max: 4000.0, step: 100.0, def: 1400.0, unit: "点/秒",
        hint: "",
    },
    ParamMeta {
        group: GROUPS[1],
        key: "pupilSize",
        label: "好奇时瞳孔多大",
        kind: ParamKind::Range,
        min: 100.0, max: 300.0, step: 10.0, def: 230.0, unit: "%",
        hint: "相对平时竖瞳的宽度。",
    },
    ParamMeta {
        group: GROUPS[1],
        key: "curiousHold",
        label: "好奇持续多久",
        kind: ParamKind::Range,
        min: 0.5, max: 6.0, step: 0.5, def: 2.0, unit: "秒",
        hint: "鼠标停下后，还要盯这么久才放松。",
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub annoy_on: bool,
    pub type_rate: f32,
    pub mash_keys: usize,
    pub pet_limit: f32,
    pub wake_annoy: f32,
    pub cross_at: f32,
    pub calm_rate: f32,
    pub thump_rate: f32,
    pub curious_on: bool,
    pub curious_speed: f32,
    pub pupil_size: f32,
    pub curious_hold: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            annoy_on: true,
            type_rate: 9.0,
            mash_keys: 4,
            pet_limit: 4.0,
            wake_annoy: 35.0,
            cross_at: 60.0,
            calm_rate: 10.0,
            thump_rate: 2.6,
            curious_on: true,
            curious_speed: 1400.0,
            pupil_size: 230.0,
            curious_hold: 2.0,
        }
    }
}

impl Tuning {
    pub fn from_json(obj: &Map<String, Value>) -> Self {
        let mut t = Self::default();
        let b = |k: &str, d: bool| obj.get(k).and_then(Value::as_bool).unwrap_or(d);
        let f = |k: &str, d: f32| obj.get(k).and_then(Value::as_f64).map(|v| v as f32).unwrap_or(d);
        t.annoy_on = b("annoyOn", t.annoy_on);
        t.type_rate = f("typeRate", t.type_rate);
        t.mash_keys = (f("mashKeys", t.mash_keys as f32).round() as usize).max(1);
        t.pet_limit = f("petLimit", t.pet_limit);
        t.wake_annoy = f("wakeAnnoy", t.wake_annoy);
        t.cross_at = f("crossAt", t.cross_at);
        t.calm_rate = f("calmRate", t.calm_rate);
        t.thump_rate = f("thumpRate", t.thump_rate);
        t.curious_on = b("curiousOn", t.curious_on);
        t.curious_speed = f("curiousSpeed", t.curious_speed);
        t.pupil_size = f("pupilSize", t.pupil_size);
        t.curious_hold = f("curiousHold", t.curious_hold);
        t
    }

    pub fn get_value(&self, key: &str) -> f32 {
        match key {
            "annoyOn" => if self.annoy_on { 1.0 } else { 0.0 },
            "typeRate" => self.type_rate,
            "mashKeys" => self.mash_keys as f32,
            "petLimit" => self.pet_limit,
            "wakeAnnoy" => self.wake_annoy,
            "crossAt" => self.cross_at,
            "calmRate" => self.calm_rate,
            "thumpRate" => self.thump_rate,
            "curiousOn" => if self.curious_on { 1.0 } else { 0.0 },
            "curiousSpeed" => self.curious_speed,
            "pupilSize" => self.pupil_size,
            "curiousHold" => self.curious_hold,
            _ => 0.0,
        }
    }

    pub fn set_value(&mut self, key: &str, v: f32) -> Value {
        match key {
            "annoyOn" => { self.annoy_on = v >= 0.5; Value::Bool(self.annoy_on) }
            "typeRate" => { self.type_rate = v; Value::from(v.round() as i64) }
            "mashKeys" => { self.mash_keys = (v.round() as usize).max(1); Value::from(self.mash_keys as i64) }
            "petLimit" => { self.pet_limit = v; Value::from(v.round() as i64) }
            "wakeAnnoy" => { self.wake_annoy = v; Value::from(v.round() as i64) }
            "crossAt" => { self.cross_at = v; Value::from(v.round() as i64) }
            "calmRate" => { self.calm_rate = v; Value::from(v.round() as i64) }
            "thumpRate" => {
                let r = (v * 10.0).round() / 10.0;
                self.thump_rate = r;
                Value::from((r as f64 * 10.0).round() / 10.0)
            }
            "curiousOn" => { self.curious_on = v >= 0.5; Value::Bool(self.curious_on) }
            "curiousSpeed" => { self.curious_speed = v; Value::from(v.round() as i64) }
            "pupilSize" => { self.pupil_size = v; Value::from(v.round() as i64) }
            "curiousHold" => {
                let r = (v * 2.0).round() / 2.0;
                self.curious_hold = r;
                Value::from((r as f64 * 10.0).round() / 10.0)
            }
            _ => Value::Null,
        }
    }
}
