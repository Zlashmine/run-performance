-- ── 0. Ensure system user exists (idempotent) ────────────────────────────────
INSERT INTO users (id, google_id, email, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'system',
    'system@run-performance.internal',
    now()
) ON CONFLICT (id) DO NOTHING;

-- ── 1. Challenge ─────────────────────────────────────────────────────────────
INSERT INTO challenges
    (id, user_id, name, description, is_recurring, status, is_public, created_at, updated_at)
VALUES (
    'a47eeaaf-6217-44a3-bec9-107465dad5ad',
    '00000000-0000-0000-0000-000000000001',
    '10-Workout Speed & Weight-Loss Running Test',
    'A 10-workout running challenge for experienced runners who want to lose weight and improve performance with two focused runs per week. The program alternates shorter faster efforts around 5–6 km with longer slower endurance runs up to 12 km, ending with a performance test.',
    false,
    'active',
    true,
    now(), now()
) ON CONFLICT (id) DO NOTHING;

-- ── 2. Workouts ──────────────────────────────────────────────────────────────
INSERT INTO challenge_workouts
    (id, challenge_id, position, name, description, created_at, updated_at)
VALUES
    ('9db831bd-f32e-4663-8818-28a1779eafe9', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  1, 'Fast 5K Baseline Run',        'Run 5 km at a strong but controlled pace to establish your starting performance level.',       now(), now()),
    ('25cfde3e-8623-4a36-ba9b-3920a316c9c4', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  2, 'Long Easy Run — 7K',          'Run 7 km at a slower endurance pace to build aerobic capacity and burn calories.',              now(), now()),
    ('b0be2854-687a-4390-a221-d120a9cbb212', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  3, 'Fast 5.5K Progress Run',      'Run 5.5 km at a strong pace to extend your speed endurance.',                                  now(), now()),
    ('3e65bafb-3826-484d-bde7-e2d60d90de4b', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  4, 'Long Easy Run — 8K',          'Run 8 km at a comfortable slower pace to increase total endurance work.',                      now(), now()),
    ('1ad10f8e-729a-48e0-a444-352dbde62d9b', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  5, 'Fast 6K Strength Run',        'Run 6 km at a challenging pace to improve sustained running performance.',                     now(), now()),
    ('33061dd6-2df4-4d40-8f13-1bde81923b0d', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  6, 'Long Easy Run — 9.5K',        'Run 9.5 km at a slower endurance pace to build stamina and support weight loss.',               now(), now()),
    ('c3743b2a-5986-4862-940e-2ccf76061d45', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  7, 'Fast 5K Sharpening Run',      'Run 5 km faster than your earlier efforts to sharpen speed before the final build.',            now(), now()),
    ('0dec5acc-ca89-414d-ba50-df9126eb9844', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  8, 'Long Easy Run — 11K',         'Run 11 km at a controlled slower pace to prepare for the longest endurance effort.',            now(), now()),
    ('ea12f01b-762f-4851-9720-0963af61e4f4', 'a47eeaaf-6217-44a3-bec9-107465dad5ad',  9, 'Long Easy Run — 12K',         'Run 12 km at a slower endurance pace to complete the peak distance of the program.',           now(), now()),
    ('509e0db6-9577-4e92-b0a9-225b491ec907', 'a47eeaaf-6217-44a3-bec9-107465dad5ad', 10, 'Final 6K Performance Test',   'Run 6 km as a focused performance test and aim to hold close to 5:00 per km pace.',            now(), now())
ON CONFLICT (id) DO NOTHING;

-- ── 3. Requirements ──────────────────────────────────────────────────────────
INSERT INTO challenge_workout_requirements
    (id, challenge_workout_id, requirement_type, value, params)
VALUES
    -- W1: Fast 5K Baseline Run
    (gen_random_uuid(), '9db831bd-f32e-4663-8818-28a1779eafe9', 'activity_type_is',     NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '9db831bd-f32e-4663-8818-28a1779eafe9', 'distance_longer_than', 5.0,   '{}'),
    (gen_random_uuid(), '9db831bd-f32e-4663-8818-28a1779eafe9', 'pace_faster_than',     330.0, '{}'),

    -- W2: Long Easy Run — 7K
    (gen_random_uuid(), '25cfde3e-8623-4a36-ba9b-3920a316c9c4', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '25cfde3e-8623-4a36-ba9b-3920a316c9c4', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '25cfde3e-8623-4a36-ba9b-3920a316c9c4', 'distance_longer_than',         7.0,   '{}'),
    (gen_random_uuid(), '25cfde3e-8623-4a36-ba9b-3920a316c9c4', 'pace_slower_than',             390.0, '{}'),

    -- W3: Fast 5.5K Progress Run
    (gen_random_uuid(), 'b0be2854-687a-4390-a221-d120a9cbb212', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'b0be2854-687a-4390-a221-d120a9cbb212', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), 'b0be2854-687a-4390-a221-d120a9cbb212', 'distance_longer_than',         5.5,   '{}'),
    (gen_random_uuid(), 'b0be2854-687a-4390-a221-d120a9cbb212', 'pace_faster_than',             325.0, '{}'),

    -- W4: Long Easy Run — 8K
    (gen_random_uuid(), '3e65bafb-3826-484d-bde7-e2d60d90de4b', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '3e65bafb-3826-484d-bde7-e2d60d90de4b', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '3e65bafb-3826-484d-bde7-e2d60d90de4b', 'distance_longer_than',         8.0,   '{}'),
    (gen_random_uuid(), '3e65bafb-3826-484d-bde7-e2d60d90de4b', 'pace_slower_than',             390.0, '{}'),

    -- W5: Fast 6K Strength Run
    (gen_random_uuid(), '1ad10f8e-729a-48e0-a444-352dbde62d9b', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '1ad10f8e-729a-48e0-a444-352dbde62d9b', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '1ad10f8e-729a-48e0-a444-352dbde62d9b', 'distance_longer_than',         6.0,   '{}'),
    (gen_random_uuid(), '1ad10f8e-729a-48e0-a444-352dbde62d9b', 'pace_faster_than',             320.0, '{}'),

    -- W6: Long Easy Run — 9.5K
    (gen_random_uuid(), '33061dd6-2df4-4d40-8f13-1bde81923b0d', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '33061dd6-2df4-4d40-8f13-1bde81923b0d', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '33061dd6-2df4-4d40-8f13-1bde81923b0d', 'distance_longer_than',         9.5,   '{}'),
    (gen_random_uuid(), '33061dd6-2df4-4d40-8f13-1bde81923b0d', 'pace_slower_than',             390.0, '{}'),

    -- W7: Fast 5K Sharpening Run
    (gen_random_uuid(), 'c3743b2a-5986-4862-940e-2ccf76061d45', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'c3743b2a-5986-4862-940e-2ccf76061d45', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), 'c3743b2a-5986-4862-940e-2ccf76061d45', 'distance_longer_than',         5.0,   '{}'),
    (gen_random_uuid(), 'c3743b2a-5986-4862-940e-2ccf76061d45', 'pace_faster_than',             315.0, '{}'),

    -- W8: Long Easy Run — 11K
    (gen_random_uuid(), '0dec5acc-ca89-414d-ba50-df9126eb9844', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '0dec5acc-ca89-414d-ba50-df9126eb9844', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '0dec5acc-ca89-414d-ba50-df9126eb9844', 'distance_longer_than',         11.0,  '{}'),
    (gen_random_uuid(), '0dec5acc-ca89-414d-ba50-df9126eb9844', 'pace_slower_than',             390.0, '{}'),

    -- W9: Long Easy Run — 12K
    (gen_random_uuid(), 'ea12f01b-762f-4851-9720-0963af61e4f4', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'ea12f01b-762f-4851-9720-0963af61e4f4', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), 'ea12f01b-762f-4851-9720-0963af61e4f4', 'distance_longer_than',         12.0,  '{}'),
    (gen_random_uuid(), 'ea12f01b-762f-4851-9720-0963af61e4f4', 'pace_slower_than',             390.0, '{}'),

    -- W10: Final 6K Performance Test
    (gen_random_uuid(), '509e0db6-9577-4e92-b0a9-225b491ec907', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '509e0db6-9577-4e92-b0a9-225b491ec907', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '509e0db6-9577-4e92-b0a9-225b491ec907', 'distance_longer_than',         6.0,   '{}'),
    (gen_random_uuid(), '509e0db6-9577-4e92-b0a9-225b491ec907', 'pace_faster_than',             305.0, '{}')
;
