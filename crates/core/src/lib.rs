pub mod behavior;
pub mod cat;
pub mod coats;
pub mod geometry;
pub mod keyboard;
pub mod math;
pub mod tuning;

#[cfg(test)]
mod tests {
    use crate::behavior::{HitZone, Scene};
    use crate::coats::{Look, COATS};

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
        // annoy_now while asleep must wake the cat and immediately trigger cross state
        s.annoy_now();
        s.step(1.0 / 60.0);
        assert!(!s.asleep && s.is_cross());
        // curious_now must override cross state cleanly
        s.curious_now(3.0);
        s.step(1.0 / 60.0);
        assert!(!s.is_cross());
        // annoy_now right after curious_now must override curiosity immediately
        s.annoy_now();
        s.step(1.0 / 60.0);
        assert!(s.is_cross());
    }
}
