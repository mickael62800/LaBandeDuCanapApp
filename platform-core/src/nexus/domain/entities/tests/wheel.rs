use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::nexus::domain::entities::wheel::is_memorable_case;
use crate::nexus::domain::entities::wheel::spin_with_rng;
use crate::nexus::domain::entities::wheel::WHEEL_CASES;

#[test]
fn wheel_has_ten_cases_and_weights_sum_to_100() {
    assert_eq!(WHEEL_CASES.len(), 10);
    let total: u32 = WHEEL_CASES.iter().map(|c| c.weight).sum();
    assert_eq!(total, 100);
}

#[test]
fn historical_payouts_and_weights_are_preserved() {
    // Valeurs EXACTES de l'ancien module wheel (commit ff6e8a46^).
    let expected: &[(&str, i64, u32)] = &[
        ("blanche", 0, 25),
        ("pq", 50, 20),
        ("sieste", 200, 15),
        ("colis", 500, 12),
        ("trefle", 1000, 10),
        ("couronne", 1500, 7),
        ("ruine", -500, 5),
        ("jackpot", 5000, 3),
        ("bombe", -2000, 2),
        ("licorne", 10000, 1),
    ];
    for (i, (key, payout, weight)) in expected.iter().enumerate() {
        assert_eq!(WHEEL_CASES[i].key, *key);
        assert_eq!(WHEEL_CASES[i].payout, *payout);
        assert_eq!(WHEEL_CASES[i].weight, *weight);
    }
}

#[test]
fn case_keys_are_unique() {
    let mut keys: Vec<&str> = WHEEL_CASES.iter().map(|c| c.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), WHEEL_CASES.len());
}

#[test]
fn spin_is_deterministic_with_seeded_rng() {
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut rng2 = StdRng::seed_from_u64(42);
    for _ in 0..50 {
        assert_eq!(spin_with_rng(&mut rng1), spin_with_rng(&mut rng2));
    }
}

#[test]
fn spin_outcome_matches_case_at_index() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..100 {
        let out = spin_with_rng(&mut rng);
        assert_eq!(out.case, WHEEL_CASES[out.case_index]);
    }
}

#[test]
fn distribution_roughly_follows_weights() {
    // 100 000 spins seedes : chaque case doit sortir, et la case la plus
    // ponderee (blanche, 25%) doit sortir plus que la licorne (1%).
    let mut rng = StdRng::seed_from_u64(1337);
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for _ in 0..100_000 {
        let out = spin_with_rng(&mut rng);
        *counts.entry(out.case.key).or_default() += 1;
    }
    assert_eq!(counts.len(), 10, "toutes les cases doivent sortir");
    let blanche = counts["blanche"];
    let licorne = counts["licorne"];
    assert!(blanche > 20_000 && blanche < 30_000, "blanche={blanche}");
    assert!(licorne > 500 && licorne < 1_600, "licorne={licorne}");
    assert!(blanche > licorne * 10);
}

#[test]
fn memorable_cases_are_jackpot_licorne_bombe() {
    assert!(is_memorable_case("jackpot"));
    assert!(is_memorable_case("licorne"));
    assert!(is_memorable_case("bombe"));
    assert!(!is_memorable_case("blanche"));
    assert!(!is_memorable_case("ruine"));
    assert!(!is_memorable_case("trefle"));
}
