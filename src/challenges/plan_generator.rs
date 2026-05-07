/// Generates structured workout plans from a goal type.
///
/// All paces are in M.SS float format (5.41 = 5 min 41 sec per km).
/// All distances are in kilometres (as f64).

use super::models::GenerateChallengeRequest;
use super::requirement_type::RequirementType;

// ─── Public types ────────────────────────────────────────────────────────────

pub struct GeneratedWorkout {
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub requirements: Vec<GeneratedRequirement>,
}

pub struct GeneratedRequirement {
    pub requirement_type: RequirementType,
    pub value: Option<f64>,
    pub params: serde_json::Value,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn generate_plan(req: &GenerateChallengeRequest) -> (String, Vec<GeneratedWorkout>) {
    let target = req.target_pace_mss.unwrap_or_else(|| default_pace(&req.goal_type));
    let weeks = req.weeks.unwrap_or_else(|| default_weeks(&req.goal_type));
    let description = format_description(&req.goal_type, target, weeks);

    let workouts = match req.goal_type.as_str() {
        "5k_improvement" => build_5k_plan(target, weeks),
        _ => build_half_marathon_plan(target, weeks),
    };

    (description, workouts)
}

// ─── Pace helpers ────────────────────────────────────────────────────────────

/// Convert M.SS float to total seconds.  5.41 → 5*60 + 41 = 341.
fn mss_to_sec(mss: f64) -> f64 {
    let mins = mss.floor();
    let secs = (mss - mins) * 100.0;
    mins * 60.0 + secs
}

/// Convert total seconds to M.SS float.  341 → 5.41.
fn sec_to_mss(sec: f64) -> f64 {
    let mins = (sec / 60.0).floor();
    let secs = sec % 60.0;
    mins + secs / 100.0
}

/// Format M.SS pace as "M:SS/km" for display in workout description.
fn fmt_pace(mss: f64) -> String {
    let mins = mss.floor() as u32;
    let secs = ((mss - mss.floor()) * 100.0).round() as u32;
    format!("{mins}:{secs:02}/km")
}

/// Add `offset_sec` seconds to a M.SS pace (slower = higher value).
fn pace_plus(mss: f64, offset_sec: f64) -> f64 {
    sec_to_mss(mss_to_sec(mss) + offset_sec)
}

fn default_pace(goal_type: &str) -> f64 {
    match goal_type {
        "5k_improvement" => 5.00,  // 5:00/km
        _ => 5.41,                 // ~5:41/km → 2h half marathon
    }
}

fn default_weeks(goal_type: &str) -> u32 {
    match goal_type {
        "5k_improvement" => 6,
        _ => 12,
    }
}

fn format_description(goal_type: &str, target: f64, weeks: u32) -> String {
    let pace_str = fmt_pace(target);
    match goal_type {
        "5k_improvement" => format!(
            "{}-week 5 km improvement plan targeting {}", weeks, pace_str
        ),
        _ => format!(
            "{}-week half marathon training plan targeting {} pace", weeks, pace_str
        ),
    }
}

// ─── Requirement builder helpers ─────────────────────────────────────────────

fn dist_req(km: f64) -> GeneratedRequirement {
    GeneratedRequirement {
        requirement_type: RequirementType::DistanceLongerThan,
        value: Some(km),
        params: serde_json::json!({}),
    }
}

fn pace_req(mss: f64) -> GeneratedRequirement {
    GeneratedRequirement {
        requirement_type: RequirementType::PaceFasterThan,
        value: Some(mss),
        params: serde_json::json!({}),
    }
}

fn activity_type_req(activity_type: &str) -> GeneratedRequirement {
    GeneratedRequirement {
        requirement_type: RequirementType::ActivityTypeIs,
        value: None,
        params: serde_json::json!({ "activity_type_is": activity_type }),
    }
}

fn workout(
    pos: i32,
    name: impl Into<String>,
    desc: impl Into<String>,
    reqs: Vec<GeneratedRequirement>,
) -> GeneratedWorkout {
    GeneratedWorkout {
        name: name.into(),
        description: Some(desc.into()),
        position: pos,
        requirements: reqs,
    }
}

// ─── Half Marathon plan ──────────────────────────────────────────────────────

fn build_half_marathon_plan(target: f64, weeks: u32) -> Vec<GeneratedWorkout> {
    let easy = pace_plus(target, 90.0);
    let long_run = pace_plus(target, 60.0);
    let tempo = pace_plus(target, 30.0);

    let easy_str = fmt_pace(easy);
    let long_str = fmt_pace(long_run);
    let tempo_str = fmt_pace(tempo);
    let target_str = fmt_pace(target);

    // Number of weeks to allocate per phase
    let base_weeks = ((weeks as f64 * 0.25).round() as u32).max(2);
    let build_weeks = ((weeks as f64 * 0.25).round() as u32).max(2);
    let peak_weeks = ((weeks as f64 * 0.25).round() as u32).max(2);
    let taper_weeks = weeks.saturating_sub(base_weeks + build_weeks + peak_weeks + 1).max(1);

    let mut workouts: Vec<GeneratedWorkout> = Vec::new();
    let mut pos = 1i32;

    // ── Phase 1: Base ──────────────────────────────────────────────────────
    for w in 1..=base_weeks {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 5 km"),
            format!("Base phase. Easy effort at {easy_str}."),
            vec![activity_type_req("run"), dist_req(5.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 7 km"),
            format!("Build your aerobic base. Target pace {easy_str}."),
            vec![activity_type_req("run"), dist_req(7.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Long Run 10 km"),
            format!("Weekend long run at {long_str}. Keep it conversational."),
            vec![activity_type_req("run"), dist_req(10.0)],
        )); pos += 1;
    }

    // ── Phase 2: Build ─────────────────────────────────────────────────────
    let build_start = base_weeks + 1;
    for w in build_start..=(build_start + build_weeks - 1) {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 6 km"),
            format!("Recovery effort at {easy_str}."),
            vec![activity_type_req("run"), dist_req(6.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Tempo Run 4 km"),
            format!("Tempo effort at {tempo_str}. Comfortably hard."),
            vec![activity_type_req("run"), dist_req(4.0), pace_req(tempo)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 5 km"),
            format!("Easy recovery run at {easy_str}."),
            vec![activity_type_req("run"), dist_req(5.0)],
        )); pos += 1;
        let long_km = 12.0 + (w - build_start) as f64 * 2.0;
        workouts.push(workout(
            pos, format!("{week_label} · Long Run {long_km:.0} km"),
            format!("Weekend long run at {long_str}."),
            vec![activity_type_req("run"), dist_req(long_km)],
        )); pos += 1;
    }

    // ── Phase 3: Peak ──────────────────────────────────────────────────────
    let peak_start = build_start + build_weeks;
    for w in peak_start..=(peak_start + peak_weeks - 1) {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 6 km"),
            format!("Easy effort at {easy_str}."),
            vec![activity_type_req("run"), dist_req(6.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Tempo Run 6 km"),
            format!("Strong tempo effort at {tempo_str}."),
            vec![activity_type_req("run"), dist_req(6.0), pace_req(tempo)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 5 km"),
            format!("Recovery run at {easy_str}."),
            vec![activity_type_req("run"), dist_req(5.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Long Run 18 km"),
            format!("Peak long run at {long_str}. Your longest run of the plan."),
            vec![activity_type_req("run"), dist_req(18.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Recovery Run 3 km"),
            format!("Very easy at {easy_str}. Flush the legs."),
            vec![activity_type_req("run"), dist_req(3.0)],
        )); pos += 1;
    }

    // ── Phase 4: Taper ─────────────────────────────────────────────────────
    let taper_start = peak_start + peak_weeks;
    for w in taper_start..=(taper_start + taper_weeks - 1) {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 5 km"),
            format!("Taper begins. Easy at {easy_str}. Save the legs."),
            vec![activity_type_req("run"), dist_req(5.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Tempo Run 3 km"),
            format!("Short tempo at {tempo_str} to stay sharp."),
            vec![activity_type_req("run"), dist_req(3.0), pace_req(tempo)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Long Run 12 km"),
            format!("Reduced long run at {long_str} during taper."),
            vec![activity_type_req("run"), dist_req(12.0)],
        )); pos += 1;
    }

    // ── Race Week ──────────────────────────────────────────────────────────
    let race_week = weeks;
    workouts.push(workout(
        pos, format!("Week {race_week} · Easy Run 3 km"),
        format!("Race week shakeout at {easy_str}. Keep it gentle."),
        vec![activity_type_req("run"), dist_req(3.0)],
    )); pos += 1;
    workouts.push(workout(
        pos, format!("Week {race_week} · Shakeout 2 km"),
        format!("Light 2 km at race pace {target_str} to prime the legs."),
        vec![activity_type_req("run"), dist_req(2.0), pace_req(target)],
    )); pos += 1;
    workouts.push(workout(
        pos, format!("Week {race_week} · Race Day 21.1 km"),
        format!("Race day! Target pace {target_str}. Run your race."),
        vec![activity_type_req("run"), dist_req(21.1), pace_req(target)],
    ));

    workouts
}

// ─── 5 km improvement plan ───────────────────────────────────────────────────

fn build_5k_plan(target: f64, weeks: u32) -> Vec<GeneratedWorkout> {
    let easy = pace_plus(target, 75.0); // +1:15/km
    let tempo = pace_plus(target, 15.0); // +15 sec/km

    let easy_str = fmt_pace(easy);
    let tempo_str = fmt_pace(tempo);
    let target_str = fmt_pace(target);

    let base_weeks = ((weeks as f64 * 0.33).round() as u32).max(2);
    let build_weeks = ((weeks as f64 * 0.33).round() as u32).max(2);

    let mut workouts: Vec<GeneratedWorkout> = Vec::new();
    let mut pos = 1i32;

    // ── Base ───────────────────────────────────────────────────────────────
    for w in 1..=base_weeks {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 3 km"),
            format!("Base phase. Easy effort at {easy_str}."),
            vec![activity_type_req("run"), dist_req(3.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 4 km"),
            format!("Build aerobic base at {easy_str}."),
            vec![activity_type_req("run"), dist_req(4.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 5 km"),
            format!("Weekend run at {easy_str}. Build your base."),
            vec![activity_type_req("run"), dist_req(5.0)],
        )); pos += 1;
    }

    // ── Build ──────────────────────────────────────────────────────────────
    let build_start = base_weeks + 1;
    for w in build_start..=(build_start + build_weeks - 1) {
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 4 km"),
            format!("Recovery effort at {easy_str}."),
            vec![activity_type_req("run"), dist_req(4.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Tempo Run 2 km"),
            format!("Hard tempo at {tempo_str}. Push the effort."),
            vec![activity_type_req("run"), dist_req(2.0), pace_req(tempo)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 3 km"),
            format!("Easy recovery run at {easy_str}."),
            vec![activity_type_req("run"), dist_req(3.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Long Run 6 km"),
            format!("Longest run of the week at {easy_str}."),
            vec![activity_type_req("run"), dist_req(6.0)],
        )); pos += 1;
    }

    // ── Peak / Taper / Race ────────────────────────────────────────────────
    let final_start = build_start + build_weeks;
    let remaining_weeks = weeks.saturating_sub(final_start - 1).max(1);
    if remaining_weeks >= 2 {
        let w = final_start;
        let week_label = format!("Week {w}");
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 3 km"),
            format!("Taper. Easy at {easy_str}."),
            vec![activity_type_req("run"), dist_req(3.0)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Tempo Run 2 km"),
            format!("Sharp tempo at {tempo_str}."),
            vec![activity_type_req("run"), dist_req(2.0), pace_req(tempo)],
        )); pos += 1;
        workouts.push(workout(
            pos, format!("{week_label} · Easy Run 4 km"),
            format!("Remaining easy volume at {easy_str}."),
            vec![activity_type_req("run"), dist_req(4.0)],
        )); pos += 1;
    }

    // Race week
    let race_week = weeks;
    workouts.push(workout(
        pos, format!("Week {race_week} · Easy Run 2 km"),
        format!("Race week. Very easy at {easy_str}."),
        vec![activity_type_req("run"), dist_req(2.0)],
    )); pos += 1;
    workouts.push(workout(
        pos, format!("Week {race_week} · Shakeout 1 km"),
        format!("1 km at race pace {target_str}."),
        vec![activity_type_req("run"), dist_req(1.0), pace_req(target)],
    )); pos += 1;
    workouts.push(workout(
        pos, format!("Week {race_week} · Race Day 5 km"),
        format!("Race day! Target pace {target_str}. Run your best."),
        vec![activity_type_req("run"), dist_req(5.0), pace_req(target)],
    ));

    workouts
}
