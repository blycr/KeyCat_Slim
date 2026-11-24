//! Local cat memory, bond progression, and calendar festival detection.
//!
//! Stores only aggregate daily counts (keystrokes, pets, naps, breaks) and bond points —
//! never which keys were pressed or any typed text.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

const KEEP_DAYS: usize = 35;

/// Bond thresholds and Chinese level names.
pub const BOND_LEVELS: &[(f32, &str)] = &[
    (0.0, "刚认识"),
    (15.0, "混熟了"),
    (50.0, "亲近"),
    (120.0, "黏你"),
    (300.0, "家人"),
];

/// Lunar New Year dates (2026..2035) for the Spring Festival red scarf.
const CNY_DATES: &[(i32, u32, u32)] = &[
    (2026, 2, 17),
    (2027, 2, 6),
    (2028, 1, 26),
    (2029, 2, 13),
    (2030, 2, 3),
    (2031, 1, 23),
    (2032, 2, 11),
    (2033, 1, 31),
    (2034, 2, 19),
    (2035, 2, 8),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Festival {
    None,
    Xmas,
    Halloween,
    Cny,
    Birthday,
}

impl Festival {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Xmas => "xmas",
            Self::Halloween => "halloween",
            Self::Cny => "cny",
            Self::Birthday => "birthday",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DayStats {
    pub keys: f64,
    pub pets: f64,
    pub pet_sec: f64,
    pub nap_sec: f64,
    pub pushes: f64,
    pub breaks: f64,
    pub focus: f64,
    pub blinks: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct LocalClock {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: f32, // 0.0 .. 24.0
}

impl Default for LocalClock {
    fn default() -> Self {
        Self { year: 2026, month: 1, day: 1, hour: 12.0 }
    }
}

impl LocalClock {
    pub fn day_key(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn is_winter(&self) -> bool {
        matches!(self.month, 12 | 1 | 2)
    }
}

pub struct LifeMemory {
    pub first_met: String,
    pub bond: f32,
    pub days: BTreeMap<String, DayStats>,
    pub clock: LocalClock,
    clock_set: bool,
    dirty: bool,
    last_save_t: f32,
}

impl LifeMemory {
    pub fn new() -> Self {
        let clock = LocalClock::default();
        Self {
            first_met: clock.day_key(),
            bond: 0.0,
            days: BTreeMap::new(),
            clock,
            clock_set: false,
            dirty: false,
            last_save_t: 0.0,
        }
    }

    pub fn set_clock(&mut self, year: i32, month: u32, day: u32, hour: f32) {
        let c = LocalClock {
            year: year.clamp(2000, 2200),
            month: month.clamp(1, 12),
            day: day.clamp(1, 31),
            hour: hour.clamp(0.0, 23.999),
        };
        if !self.clock_set && self.days.is_empty() && self.bond == 0.0 {
            self.first_met = c.day_key();
        }
        self.clock = c;
        self.clock_set = true;
    }

    pub fn load_json(&mut self, json: &str) {
        let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(json) else { return };
        if let Some(s) = obj.get("firstMet").and_then(Value::as_str) {
            if is_day_key(s) {
                self.first_met = s.to_string();
            }
        }
        if let Some(b) = obj.get("bond").and_then(Value::as_f64) {
            if b.is_finite() && b >= 0.0 {
                self.bond = b as f32;
            }
        }
        if let Some(Value::Object(days_obj)) = obj.get("days") {
            let mut merged = BTreeMap::new();
            for (k, v) in days_obj {
                if !is_day_key(k) {
                    continue;
                }
                if let Value::Object(d) = v {
                    let g = |name: &str| {
                        d.get(name)
                            .and_then(Value::as_f64)
                            .filter(|x| x.is_finite() && *x >= 0.0)
                            .unwrap_or(0.0)
                    };
                    merged.insert(
                        k.clone(),
                        DayStats {
                            keys: g("keys"),
                            pets: g("pets"),
                            pet_sec: g("petSec"),
                            nap_sec: g("napSec"),
                            pushes: g("pushes"),
                            breaks: g("breaks"),
                            focus: g("focus"),
                            blinks: g("blinks"),
                        },
                    );
                }
            }
            // Merge any counts gathered before load arrived.
            for (k, d) in &self.days {
                let entry = merged.entry(k.clone()).or_default();
                entry.keys += d.keys;
                entry.pets += d.pets;
                entry.pet_sec += d.pet_sec;
                entry.nap_sec += d.nap_sec;
                entry.pushes += d.pushes;
                entry.breaks += d.breaks;
                entry.focus += d.focus;
                entry.blinks += d.blinks;
            }
            while merged.len() > KEEP_DAYS {
                if let Some(oldest) = merged.keys().next().cloned() {
                    merged.remove(&oldest);
                }
            }
            self.days = merged;
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = Map::new();
        root.insert("v".into(), Value::from(1));
        root.insert("firstMet".into(), Value::from(self.first_met.clone()));
        root.insert("bond".into(), Value::from((self.bond * 100.0).round() / 100.0));
        let mut days_map = Map::new();
        for (k, d) in &self.days {
            let mut dm = Map::new();
            dm.insert("keys".into(), Value::from(d.keys.round() as i64));
            dm.insert("pets".into(), Value::from(d.pets.round() as i64));
            dm.insert("petSec".into(), Value::from((d.pet_sec * 10.0).round() / 10.0));
            dm.insert("napSec".into(), Value::from((d.nap_sec * 10.0).round() / 10.0));
            dm.insert("pushes".into(), Value::from(d.pushes.round() as i64));
            dm.insert("breaks".into(), Value::from(d.breaks.round() as i64));
            dm.insert("focus".into(), Value::from(d.focus.round() as i64));
            dm.insert("blinks".into(), Value::from(d.blinks.round() as i64));
            days_map.insert(k.clone(), Value::Object(dm));
        }
        root.insert("days".into(), Value::Object(days_map));
        serde_json::to_string(&Value::Object(root)).unwrap_or_else(|_| "{}".into())
    }

    fn today_mut(&mut self) -> &mut DayStats {
        let k = self.clock.day_key();
        if !self.days.contains_key(&k) {
            self.days.insert(k.clone(), DayStats::default());
            while self.days.len() > KEEP_DAYS {
                if let Some(oldest) = self.days.keys().next().cloned() {
                    self.days.remove(&oldest);
                }
            }
        }
        self.dirty = true;
        self.days.get_mut(&k).unwrap()
    }

    pub fn add_keys(&mut self, n: f64) { self.today_mut().keys += n; }
    pub fn add_pets(&mut self, n: f64) { self.today_mut().pets += n; }
    pub fn add_pet_sec(&mut self, n: f64) { self.today_mut().pet_sec += n; }
    pub fn add_nap_sec(&mut self, n: f64) { self.today_mut().nap_sec += n; }
    pub fn add_pushes(&mut self, n: f64) { self.today_mut().pushes += n; }
    pub fn add_breaks(&mut self, n: f64) { self.today_mut().breaks += n; }
    pub fn add_focus(&mut self, n: f64) { self.today_mut().focus += n; }
    pub fn add_blinks(&mut self, n: f64) { self.today_mut().blinks += n; }

    pub fn add_bond(&mut self, pts: f32) {
        if pts > 0.0 {
            self.bond += pts;
            self.dirty = true;
        }
    }

    /// Returns `(level 1..=5, name)`.
    pub fn level(&self) -> (usize, &'static str) {
        let mut idx = 0;
        while idx + 1 < BOND_LEVELS.len() && self.bond >= BOND_LEVELS[idx + 1].0 {
            idx += 1;
        }
        (idx + 1, BOND_LEVELS[idx].1)
    }

    /// Returns JSON snapshot when enough time has passed since last save (or if forced).
    pub fn take_dirty_json(&mut self, now_t: f32, force: bool) -> Option<String> {
        if !self.dirty && !force {
            return None;
        }
        if !force && now_t - self.last_save_t < 60.0 {
            return None;
        }
        self.dirty = false;
        self.last_save_t = now_t;
        Some(self.to_json())
    }

    pub fn festival(&self) -> Festival {
        if !self.clock_set {
            return Festival::None;
        }
        let m = self.clock.month;
        let d = self.clock.day;
        if m == 12 && (18..=26).contains(&d) {
            return Festival::Xmas;
        }
        if (m == 10 && d >= 29) || (m == 11 && d == 1) {
            return Festival::Halloween;
        }
        let today_jdn = civil_to_days(self.clock.year, m, d);
        for &(cy, cm, cd) in CNY_DATES {
            let diff = today_jdn - civil_to_days(cy, cm, cd);
            if (-3..8).contains(&diff) {
                return Festival::Cny;
            }
        }
        let key = self.clock.day_key();
        if key.len() == 10 && self.first_met.len() == 10
            && key[5..] == self.first_met[5..]
            && key != self.first_met
        {
            return Festival::Birthday;
        }
        Festival::None
    }
}

fn is_day_key(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Howard Hinnant's civil date to days-since-epoch conversion (exact, zero dependencies).
fn civil_to_days(y: i32, m: u32, d: u32) -> i32 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m_adj = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}
