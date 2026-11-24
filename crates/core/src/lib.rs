pub mod audio;
pub mod behavior;
pub mod cat;
pub mod coats;
pub mod geometry;
pub mod keyboard;
pub mod life;
pub mod math;
pub mod tuning;

#[cfg(test)]
mod tests {
    use crate::audio::PurrSynth;
    use crate::behavior::{HitZone, Scene};
    use crate::coats::{Look, COATS};
    use crate::life::{Festival, LifeMemory};
    use crate::tuning::{format_param_value, Tuning, TUNING_PRESETS, TUNING_SCHEMA};

    #[test]
    fn scene_boots_and_hit_tests_match_web_reference() {
        let mut s = Scene::new();
        s.set_viewport(360.0, 368.0);
        for _ in 0..10 {
            s.step(1.0 / 60.0);
        }
        assert_eq!(s.hit_test(171.0, 135.0), HitZone::Pet);
        assert_eq!(s.hit_test(171.0, 240.0), HitZone::Drag);
        assert_eq!(s.hit_test(20.0, 20.0), HitZone::None);
        assert_eq!(s.hit_test(-10.0, 50.0), HitZone::None);
    }

    #[test]
    fn key_press_tail_space_and_all_coats_work() {
        let mut s = Scene::new();
        for c in COATS {
            s.set_look(Look {
                style_3d: true,
                coat: c.id.into(),
                collar: "red".into(),
                gloves: "pink".into(),
                keyboard_color: "mint".into(),
                ..Default::default()
            });
            s.step(1.0 / 60.0);
        }
        s.on_key("KeyF", true);
        s.step(0.15);
        assert!(!s.sprites.is_empty());
        s.on_key("KeyF", false);
        s.on_key("Space", true);
        for _ in 0..20 {
            s.step(1.0 / 60.0);
        }
        s.on_key("Space", false);
        s.annoy_now();
        for _ in 0..120 {
            s.step(1.0 / 60.0);
        }
        s.sleep_now();
        for _ in 0..300 {
            s.step(1.0 / 30.0);
        }
        assert!(s.sleep > 0.7);
        s.annoy_now();
        s.step(1.0 / 60.0);
        assert!(!s.asleep && s.is_cross());
        s.curious_now(3.0);
        s.step(1.0 / 60.0);
        assert!(!s.is_cross());
        s.annoy_now();
        s.step(1.0 / 60.0);
        assert!(s.is_cross());
    }

    #[test]
    fn purr_suppresses_pet_annoyance_and_audio_synth_works() {
        let mut s = Scene::new();
        s.tuning.purr_after = 0.5;
        s.tuning.pet_limit = 2.0;
        s.on_pet(true, 171.0);
        for _ in 0..240 {
            s.step(1.0 / 30.0);
        }
        assert!(s.purr > 0.8);
        assert!(!s.is_cross());
        s.on_pet(false, 171.0);

        let mut synth = PurrSynth::new(48_000);
        let mut buf = [0.0f32; 512];
        synth.fill_f32(&mut buf);
        assert!(buf.iter().all(|&x| x == 0.0));
        synth.set_target(true, 1.0, 0.25);
        synth.fill_f32(&mut buf);
        assert!(buf.iter().any(|&x| x.abs() > 1e-4));
        assert!(buf.iter().all(|&x| x.is_finite() && x.abs() <= 1.0));
    }

    #[test]
    fn life_memory_festivals_and_presets_work() {
        let mut life = LifeMemory::new();
        life.set_clock(2026, 12, 24, 23.8);
        assert_eq!(life.festival(), Festival::Xmas);
        life.set_clock(2026, 2, 17, 10.0);
        assert_eq!(life.festival(), Festival::Cny);
        life.set_clock(2026, 10, 31, 20.0);
        assert_eq!(life.festival(), Festival::Halloween);
        life.add_keys(42.0);
        life.add_bond(55.0);
        assert_eq!(life.level().0, 3);
        let json = life.to_json();
        let mut life2 = LifeMemory::new();
        life2.load_json(&json);
        assert_eq!(life2.level().0, 3);

        let mut t = Tuning::default();
        for i in 0..TUNING_PRESETS.len() {
            t.apply_preset(i);
            assert_eq!(t.matching_preset(), Some(i));
        }
        let night_meta = TUNING_SCHEMA.iter().find(|m| m.key == "nightAt").unwrap();
        assert_eq!(format_param_value(night_meta, 23.5), "23:30");
        assert_eq!(format_param_value(night_meta, 25.0), "01:00");
    }

    #[test]
    fn pomodoro_and_caps_lock_release_work() {
        let mut s = Scene::new();
        s.set_pomodoro(true);
        assert!(s.pomodoro);
        s.step(1.0 / 60.0);
        assert!(!s.sprites.is_empty());
        s.set_pomodoro(false);
        assert!(!s.pomodoro);

        s.on_key("CapsLock", true);
        for _ in 0..60 {
            s.step(1.0 / 30.0);
        }
        let caps_idx = s.keyboard.resolve("CapsLock").unwrap();
        assert!(!s.keyboard.keys[caps_idx].down);
    }
}
