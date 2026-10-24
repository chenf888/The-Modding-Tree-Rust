//! Headless smoke test: boot the demo game, prestige, buy, run ticks and
//! render — everything the browser does except the DOM itself.

use tmt_engine::Decimal;
use tmt_engine::{demo, game::Game, render};

#[test]
fn demo_game_smoke() {
    let mut g = Game::new(demo::game_defs());
    g.update_tmp();
    g.update_tmp();

    // Start: 10 points, "c" requires 10, exponent 0.5 → gain floor(sqrt(1)) = 1.
    assert!(g.get_can_reset("c"));
    assert_eq!(g.get_reset_gain("c"), Decimal::one());

    g.do_reset("c", false);
    // Row-0 reset zeroes points; the layer keeps its gained lollipop.
    assert_eq!(g.player.points, Decimal::zero());
    assert_eq!(g.lstate("c").points, Decimal::one());

    // Buy upgrade c11 (cost 1 lollipop).
    g.update_tmp();
    g.buy_upg("c", 11);
    assert!(g.has_upgrade("c", 11));
    assert_eq!(g.lstate("c").points, Decimal::zero());

    // Points/sec now on (1/s).
    g.update_tmp();
    assert_eq!(g.tmp.point_gen, Decimal::one());

    // 100 ticks of 50ms ≈ 5 seconds → ~5 points.
    g.last_time = 0.0;
    for i in 1..=100 {
        g.tick(i as f64 * 50.0, 1280.0, 800.0, false);
    }
    assert!(g.player.points > Decimal::one());

    // Prestige again with more points (5 → floor(sqrt(0.5)) = 0 gain, but
    // still allowed: canReset only needs baseAmount >= requires).
    g.update_tmp();
    g.do_reset("c", false);
    assert_eq!(g.lstate("c").milestones.len(), 0); // best stays 1 < 3
    // Side layers (row "side") are never layerDataReset by the cascade:
    // the a11 achievement earned during the ticks above must survive.
    assert!(g.has_achievement("a", 11), "side-layer achievements must survive prestige");

    // Milestone path: force best to 3 → milestone 0 unlocks on next tick.
    g.lstate_mut("c").best = Decimal::from(3);
    g.tick(6000.0, 1280.0, 800.0, false);
    assert!(g.has_milestone("c", 0));

    // Rendering produces a full tree first, patches afterwards.
    let first = render::render_and_diff(&mut g);
    let s = first.to_string();
    assert!(s.contains("\"full\""), "first render must be a full tree");
    let second = render::render_and_diff(&mut g);
    assert!(second.to_string().starts_with("["), "second render must be patches");

    // Save round-trip.
    let save = g.save_json();
    g.import_save(&save, true).expect("save should re-import");
    assert!(g.has_upgrade("c", 11));
    assert!(g.has_milestone("c", 0));
}

#[test]
fn endgame_freezes_game() {
    let mut g = Game::new(demo::game_defs());
    g.update_tmp();
    g.update_tmp();
    // Past the demo's endgame condition (points >= 11), keepGoing false.
    g.player.points = Decimal::from(50);
    g.last_time = 0.0;
    for i in 1..=20 {
        g.tick(i as f64 * 50.0, 1280.0, 800.0, false);
    }
    assert_eq!(g.player.time_played, 0.0, "timePlayed must freeze after endgame");
    assert_eq!(g.player.points, Decimal::from(50), "points must freeze after endgame");
    assert!(g.tmp.game_ended);

    // Keep Going resumes the loop (first tick re-binds last_time from the
    // 0.0 sentinel, the second one passes real time).
    g.player.keep_going = true;
    g.tick(1050.0, 1280.0, 800.0, false);
    g.tick(1100.0, 1280.0, 800.0, false);
    assert!(g.player.time_played > 0.0);
}

#[test]
fn challenge_flow() {
    let mut g = Game::new(demo::game_defs());
    g.update_tmp();
    g.update_tmp();

    // Enter challenge c11 (unlocked once best > 0).
    g.lstate_mut("c").best = Decimal::one();
    g.update_tmp();
    g.start_challenge("c", 11);
    assert_eq!(g.lstate("c").active_challenge, Some(11));

    // Goal: 20 points.
    g.player.points = Decimal::from(25);
    g.update_tmp();
    assert!(g.can_complete_challenge("c", 11));
    g.start_challenge("c", 11); // clicking again completes + exits
    assert_eq!(g.lstate("c").active_challenge, None);
    assert_eq!(g.challenge_completions("c", 11), 1);
}
