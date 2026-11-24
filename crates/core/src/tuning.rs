//! Behaviour tuning parameters and personality presets.

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

pub const GROUPS: [&str; 6] = [
    "不耐烦（尾巴捶桌）",
    "好奇（快速移动鼠标）",
    "亲近",
    "睡觉",
    "照顾你",
    "番茄钟",
];

pub const TUNING_SCHEMA: &[ParamMeta] = &[
    // Group 0: 不耐烦（尾巴捶桌）
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
        hint: "摸了这么久还没呼噜起来就会烦；呼噜起来以后怎么摸都不烦。越熟越能忍。",
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

    // Group 1: 好奇（快速移动鼠标）
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

    // Group 2: 亲近
    ParamMeta {
        group: GROUPS[2],
        key: "purrAfter",
        label: "摸头多久开始呼噜",
        kind: ParamKind::Range,
        min: 0.5, max: 10.0, step: 0.5, def: 1.5, unit: "秒",
        hint: "呼噜时两只前爪轮流踩奶。声音可在菜单「习性」里打开。",
    },
    ParamMeta {
        group: GROUPS[2],
        key: "blinkBack",
        label: "回应慢眨眼的概率",
        kind: ParamKind::Range,
        min: 0.0, max: 100.0, step: 5.0, def: 60.0, unit: "%",
        hint: "双击猫头就是对它慢眨眼，越熟越容易回应。",
    },
    ParamMeta {
        group: GROUPS[2],
        key: "needyAfter",
        label: "只用鼠标多久来找存在感",
        kind: ParamKind::Range,
        min: 0.0, max: 30.0, step: 1.0, def: 3.0, unit: "分钟",
        hint: "它会自己按一下键盘或拍空格再看着你。0 表示不会。",
    },

    // Group 3: 睡觉
    ParamMeta {
        group: GROUPS[3],
        key: "sprawlAfter",
        label: "睡多久后霸占键盘",
        kind: ParamKind::Range,
        min: 0.0, max: 30.0, step: 1.0, def: 3.0, unit: "分钟",
        hint: "睡着后再过这么久没人理，它会趴到键盘上睡。0 表示不会。",
    },

    // Group 4: 照顾你
    ParamMeta {
        group: GROUPS[4],
        key: "breakAfter",
        label: "连续工作多久提醒休息",
        kind: ParamKind::Range,
        min: 0.0, max: 120.0, step: 5.0, def: 50.0, unit: "分钟",
        hint: "到点会趴到键盘上拦着打字，摸摸它可再延 10 分钟。停 3 分钟以上算休息过，0 表示不提醒。",
    },
    ParamMeta {
        group: GROUPS[4],
        key: "nightOn",
        label: "深夜催睡",
        kind: ParamKind::Toggle,
        min: 0.0, max: 1.0, step: 1.0, def: 1.0, unit: "",
        hint: "过了下面的时间你还在用电脑，它会犯困打哈欠并提醒早点休息。",
    },
    ParamMeta {
        group: GROUPS[4],
        key: "nightAt",
        label: "几点算深夜",
        kind: ParamKind::Range,
        min: 21.0, max: 26.0, step: 0.5, def: 23.5, unit: "hour",
        hint: "",
    },

    // Group 5: 番茄钟
    ParamMeta {
        group: GROUPS[5],
        key: "pomoFocus",
        label: "专注多久",
        kind: ParamKind::Range,
        min: 5.0, max: 60.0, step: 5.0, def: 25.0, unit: "分钟",
        hint: "在菜单顶部开启。专注期间猫安静陪伴，不做干扰小动作。",
    },
    ParamMeta {
        group: GROUPS[5],
        key: "pomoBreak",
        label: "休息多久",
        kind: ParamKind::Range,
        min: 1.0, max: 30.0, step: 1.0, def: 5.0, unit: "分钟",
        hint: "",
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub hint: &'static str,
    pub values: &'static [(&'static str, f32)],
}

pub const TUNING_PRESETS: &[Preset] = &[
    Preset {
        name: "默认",
        hint: "平衡的猫咪脾气",
        values: &[],
    },
    Preset {
        name: "高冷",
        hint: "不太黏人，摸久了就烦，很少回应你",
        values: &[
            ("petLimit", 3.0),
            ("purrAfter", 4.0),
            ("blinkBack", 25.0),
            ("needyAfter", 0.0),
            ("typeRate", 8.0),
            ("curiousSpeed", 2000.0),
        ],
    },
    Preset {
        name: "粘人",
        hint: "怎么摸都行，很快呼噜，常来找你",
        values: &[
            ("petLimit", 30.0),
            ("purrAfter", 0.5),
            ("blinkBack", 95.0),
            ("needyAfter", 2.0),
            ("wakeAnnoy", 0.0),
            ("calmRate", 30.0),
        ],
    },
    Preset {
        name: "暴躁",
        hint: "一点就着，消气还慢",
        values: &[
            ("typeRate", 6.0),
            ("mashKeys", 3.0),
            ("petLimit", 2.0),
            ("wakeAnnoy", 80.0),
            ("crossAt", 40.0),
            ("calmRate", 5.0),
            ("thumpRate", 3.5),
            ("blinkBack", 35.0),
        ],
    },
    Preset {
        name: "胆小",
        hint: "容易被吵醒和惊到，一点动静就瞪圆眼睛",
        values: &[
            ("wakeAnnoy", 60.0),
            ("mashKeys", 3.0),
            ("pupilSize", 290.0),
            ("curiousSpeed", 900.0),
            ("blinkBack", 50.0),
            ("curiousHold", 4.0),
        ],
    },
];

pub fn format_param_value(meta: &ParamMeta, v: f32) -> String {
    if meta.unit == "hour" {
        let total_half = (v * 2.0).round() as i32;
        let h = (total_half / 2).rem_euclid(24);
        let m = if total_half % 2 != 0 { 30 } else { 0 };
        return format!("{:02}:{:02}", h, m);
    }
    let num = if (meta.step - meta.step.round()).abs() < 1e-4 {
        format!("{}", v.round() as i32)
    } else {
        format!("{:.1}", v)
    };
    if meta.unit.is_empty() {
        num
    } else {
        format!("{num} {}", meta.unit)
    }
}

/// Optional group switch key controlling whether other items in the same group are greyed out.
pub fn group_switch_key(group: &str) -> Option<&'static str> {
    if group == GROUPS[0] {
        Some("annoyOn")
    } else if group == GROUPS[1] {
        Some("curiousOn")
    } else {
        None
    }
}

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
    pub purr_after: f32,
    pub blink_back: f32,
    pub needy_after: f32,
    pub sprawl_after: f32,
    pub break_after: f32,
    pub night_on: bool,
    pub night_at: f32,
    pub pomo_focus: f32,
    pub pomo_break: f32,
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
            purr_after: 1.5,
            blink_back: 60.0,
            needy_after: 3.0,
            sprawl_after: 3.0,
            break_after: 50.0,
            night_on: true,
            night_at: 23.5,
            pomo_focus: 25.0,
            pomo_break: 5.0,
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
        t.purr_after = f("purrAfter", t.purr_after);
        t.blink_back = f("blinkBack", t.blink_back);
        t.needy_after = f("needyAfter", t.needy_after);
        t.sprawl_after = f("sprawlAfter", t.sprawl_after);
        t.break_after = f("breakAfter", t.break_after);
        t.night_on = b("nightOn", t.night_on);
        t.night_at = f("nightAt", t.night_at);
        t.pomo_focus = f("pomoFocus", t.pomo_focus);
        t.pomo_break = f("pomoBreak", t.pomo_break);
        t
    }

    pub fn to_diff_json(&self) -> Map<String, Value> {
        let def = Self::default();
        let mut out = Map::new();
        for meta in TUNING_SCHEMA {
            let cur = self.get_value(meta.key);
            let d = def.get_value(meta.key);
            if (cur - d).abs() > 1e-4 {
                let mut tmp = *self;
                let v = tmp.set_value(meta.key, cur);
                out.insert(meta.key.to_string(), v);
            }
        }
        out
    }

    pub fn apply_preset(&mut self, index: usize) {
        *self = Self::default();
        if let Some(p) = TUNING_PRESETS.get(index) {
            for &(k, v) in p.values {
                let _ = self.set_value(k, v);
            }
        }
    }

    pub fn matching_preset(&self) -> Option<usize> {
        for (i, p) in TUNING_PRESETS.iter().enumerate() {
            let mut candidate = Self::default();
            for &(k, v) in p.values {
                let _ = candidate.set_value(k, v);
            }
            let same = TUNING_SCHEMA
                .iter()
                .all(|m| (self.get_value(m.key) - candidate.get_value(m.key)).abs() < 1e-3);
            if same {
                return Some(i);
            }
        }
        None
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
            "purrAfter" => self.purr_after,
            "blinkBack" => self.blink_back,
            "needyAfter" => self.needy_after,
            "sprawlAfter" => self.sprawl_after,
            "breakAfter" => self.break_after,
            "nightOn" => if self.night_on { 1.0 } else { 0.0 },
            "nightAt" => self.night_at,
            "pomoFocus" => self.pomo_focus,
            "pomoBreak" => self.pomo_break,
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
            "purrAfter" => {
                let r = (v * 2.0).round() / 2.0;
                self.purr_after = r;
                Value::from((r as f64 * 10.0).round() / 10.0)
            }
            "blinkBack" => { self.blink_back = v; Value::from(v.round() as i64) }
            "needyAfter" => { self.needy_after = v; Value::from(v.round() as i64) }
            "sprawlAfter" => { self.sprawl_after = v; Value::from(v.round() as i64) }
            "breakAfter" => { self.break_after = v; Value::from(v.round() as i64) }
            "nightOn" => { self.night_on = v >= 0.5; Value::Bool(self.night_on) }
            "nightAt" => {
                let r = (v * 2.0).round() / 2.0;
                self.night_at = r;
                Value::from((r as f64 * 10.0).round() / 10.0)
            }
            "pomoFocus" => { self.pomo_focus = v; Value::from(v.round() as i64) }
            "pomoBreak" => { self.pomo_break = v; Value::from(v.round() as i64) }
            _ => Value::Null,
        }
    }
}
