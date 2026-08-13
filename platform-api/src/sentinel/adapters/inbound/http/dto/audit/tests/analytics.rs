use super::*;
use platform_core::sentinel::domain::entities::system::analytics::ActionDistribution;
use platform_core::sentinel::domain::entities::system::analytics::HourlyActivity;
use platform_core::sentinel::domain::entities::system::analytics::PeakActivity;
// ── AnalyticsQuery::days / limit ──

#[test]
fn days_default_is_30() {
    let q = AnalyticsQuery {
        guild_id: None,
        days: None,
        limit: None,
    };
    assert_eq!(q.days(), 30);
}

#[test]
fn days_clamp_lower_bound() {
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(0),
            limit: None
        }
        .days(),
        1
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(-5),
            limit: None
        }
        .days(),
        1
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(1),
            limit: None
        }
        .days(),
        1
    );
}

#[test]
fn days_clamp_upper_bound() {
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(90),
            limit: None
        }
        .days(),
        90
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(91),
            limit: None
        }
        .days(),
        90
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: Some(10_000),
            limit: None
        }
        .days(),
        90
    );
}

#[test]
fn days_in_range_passthrough() {
    for v in [2, 7, 30, 45, 89] {
        assert_eq!(
            AnalyticsQuery {
                guild_id: None,
                days: Some(v),
                limit: None
            }
            .days(),
            v
        );
    }
}

#[test]
fn limit_default_is_10() {
    let q = AnalyticsQuery {
        guild_id: None,
        days: None,
        limit: None,
    };
    assert_eq!(q.limit(), 10);
}

#[test]
fn limit_clamp_lower_bound() {
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: None,
            limit: Some(0)
        }
        .limit(),
        1
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: None,
            limit: Some(-1)
        }
        .limit(),
        1
    );
}

#[test]
fn limit_clamp_upper_bound() {
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: None,
            limit: Some(50)
        }
        .limit(),
        50
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: None,
            limit: Some(51)
        }
        .limit(),
        50
    );
    assert_eq!(
        AnalyticsQuery {
            guild_id: None,
            days: None,
            limit: Some(9999)
        }
        .limit(),
        50
    );
}

// ── ActionDistributionDto rounding ──

fn dist(percentage: f64) -> ActionDistribution {
    ActionDistribution {
        action: "warn".into(),
        count: 42,
        percentage,
    }
}

#[test]
fn action_distribution_rounds_to_one_decimal() {
    let d: ActionDistributionDto = dist(23.456).into();
    assert!((d.percentage - 23.5).abs() < 1e-9);
}

#[test]
fn action_distribution_small_values_round_to_zero() {
    let d: ActionDistributionDto = dist(0.001).into();
    assert_eq!(d.percentage, 0.0);
    let d: ActionDistributionDto = dist(0.049).into();
    assert_eq!(d.percentage, 0.0);
}

#[test]
fn action_distribution_rounds_half_up() {
    // 0.05 * 10 = 0.5 → round → 1 → /10 = 0.1
    let d: ActionDistributionDto = dist(0.05).into();
    assert!((d.percentage - 0.1).abs() < 1e-9);
}

#[test]
fn action_distribution_preserves_count_and_action() {
    let src = ActionDistribution {
        action: "ban_temp".into(),
        count: 123,
        percentage: 12.34,
    };
    let dto: ActionDistributionDto = src.into();
    assert_eq!(dto.action, "ban_temp");
    assert_eq!(dto.count, 123);
}

#[test]
fn action_distribution_near_hundred_rounds() {
    let d: ActionDistributionDto = dist(99.95).into();
    assert!((d.percentage - 100.0).abs() < 1e-9);
}

// ── PeakHourDto rounding + label ──

#[test]
fn peak_hour_label_format() {
    let p: PeakHourDto = PeakActivity {
        hour: 14,
        avg_messages: 10.0,
        avg_infractions: 1.0,
    }
    .into();
    assert_eq!(p.label, "14h-15h");
    assert_eq!(p.hour, 14);
}

#[test]
fn peak_hour_label_wraps_midnight() {
    let p: PeakHourDto = PeakActivity {
        hour: 23,
        avg_messages: 5.0,
        avg_infractions: 0.0,
    }
    .into();
    assert_eq!(p.label, "23h-00h");
}

#[test]
fn peak_hour_rounds_averages() {
    let p: PeakHourDto = PeakActivity {
        hour: 10,
        avg_messages: 12.3456,
        avg_infractions: 0.9876,
    }
    .into();
    assert!((p.avg_messages - 12.3).abs() < 1e-9);
    assert!((p.avg_infractions - 1.0).abs() < 1e-9);
}

// ── HeatmapPointDto day_name ──

#[test]
fn heatmap_point_day_name_known_values() {
    let p: HeatmapPointDto = HourlyActivity {
        hour: 0,
        day_of_week: 0,
        messages: 1,
        infractions: 0,
    }
    .into();
    assert_eq!(p.day_name, "Lundi");
    let p: HeatmapPointDto = HourlyActivity {
        hour: 0,
        day_of_week: 6,
        messages: 1,
        infractions: 0,
    }
    .into();
    assert_eq!(p.day_name, "Dimanche");
}

#[test]
fn heatmap_point_day_name_out_of_range() {
    let p: HeatmapPointDto = HourlyActivity {
        hour: 0,
        day_of_week: 99,
        messages: 0,
        infractions: 0,
    }
    .into();
    assert_eq!(p.day_name, "?");
    let p: HeatmapPointDto = HourlyActivity {
        hour: 0,
        day_of_week: -1,
        messages: 0,
        infractions: 0,
    }
    .into();
    assert_eq!(p.day_name, "?");
}
