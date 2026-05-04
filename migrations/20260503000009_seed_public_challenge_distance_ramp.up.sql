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
    '1d88e4c7-164d-43e2-bd98-5a1c03688d38',
    '00000000-0000-0000-0000-000000000001',
    'Distance Ramp Performance Test',
    'A structured running challenge that tests distance endurance while keeping every workout faster than 5:30 per km. The program ramps distance over time and includes short super-fast efforts to sharpen speed before the final distance test.',
    false,
    'active',
    true,
    now(), now()
) ON CONFLICT (id) DO NOTHING;

-- ── 2. Workouts ──────────────────────────────────────────────────────────────
INSERT INTO challenge_workouts
    (id, challenge_id, position, name, description, created_at, updated_at)
VALUES
    ('aa3933b6-8436-4bd0-93ae-89250b1bbce3', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  1, 'Controlled 5K Opener',    'Run 5 km faster than 5:30 per km to establish your starting distance pace.',                         now(), now()),
    ('5187fa72-3432-4cd3-b96b-f9320fde4c28', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  2, 'Short Super-Fast 3K',     'Run 3 km faster than 5:00 per km to test your short-distance speed.',                              now(), now()),
    ('3b29fcfb-5687-47bb-85ad-6359dc02c39a', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  3, 'Distance Ramp 6K',        'Run 6 km faster than 5:30 per km to start increasing your endurance load.',                         now(), now()),
    ('60550fef-4588-4bcc-8193-1fe7fb8dc2b4', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  4, 'Distance Ramp 7K',        'Run 7 km faster than 5:30 per km to continue building distance capacity.',                          now(), now()),
    ('131ea925-6c02-473b-9e45-df34e2593555', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  5, 'Short Super-Fast 4K',     'Run 4 km faster than 5:00 per km to sharpen speed under fatigue.',                                 now(), now()),
    ('26c01351-33a6-43b8-855e-889f93cc0d29', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  6, 'Distance Ramp 8K',        'Run 8 km faster than 5:30 per km to strengthen your sustained running ability.',                    now(), now()),
    ('2c3789e2-a7f7-4a2a-8ea9-43a4d6112e4d', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  7, 'Distance Ramp 9K',        'Run 9 km faster than 5:30 per km to push your endurance threshold higher.',                         now(), now()),
    ('c4666951-77f8-4c0c-9d49-3fd8ea26441f', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  8, 'Short Super-Fast 5K',     'Run 5 km faster than 5:00 per km to test your ability to hold fast pace longer.',                   now(), now()),
    ('f543902a-6ec2-4d24-bae3-0990425f2c3b', '1d88e4c7-164d-43e2-bd98-5a1c03688d38',  9, 'Distance Ramp 10K',       'Run 10 km faster than 5:30 per km to prove strong double-digit endurance.',                         now(), now()),
    ('7d8b219c-6e67-4bca-a5b6-aac9438fcf97', '1d88e4c7-164d-43e2-bd98-5a1c03688d38', 10, 'Distance Ramp 11K',       'Run 11 km faster than 5:30 per km to prepare for the final distance test.',                         now(), now()),
    ('55d617e3-aef9-4947-ab91-e9092a9a7594', '1d88e4c7-164d-43e2-bd98-5a1c03688d38', 11, 'Final Distance Test 12K', 'Run 12 km faster than 5:30 per km to complete the final distance running test.',                    now(), now())
ON CONFLICT (id) DO NOTHING;

-- ── 3. Requirements ──────────────────────────────────────────────────────────
INSERT INTO challenge_workout_requirements
    (id, challenge_workout_id, requirement_type, value, params)
VALUES
    -- W1: Controlled 5K Opener (no days_after)
    (gen_random_uuid(), 'aa3933b6-8436-4bd0-93ae-89250b1bbce3', 'activity_type_is',    NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'aa3933b6-8436-4bd0-93ae-89250b1bbce3', 'distance_longer_than', 5.0,  '{}'),
    (gen_random_uuid(), 'aa3933b6-8436-4bd0-93ae-89250b1bbce3', 'pace_faster_than',    330.0, '{}'),

    -- W2: Short Super-Fast 3K
    (gen_random_uuid(), '5187fa72-3432-4cd3-b96b-f9320fde4c28', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '5187fa72-3432-4cd3-b96b-f9320fde4c28', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '5187fa72-3432-4cd3-b96b-f9320fde4c28', 'distance_longer_than',         3.0,   '{}'),
    (gen_random_uuid(), '5187fa72-3432-4cd3-b96b-f9320fde4c28', 'pace_faster_than',             300.0, '{}'),

    -- W3: Distance Ramp 6K
    (gen_random_uuid(), '3b29fcfb-5687-47bb-85ad-6359dc02c39a', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '3b29fcfb-5687-47bb-85ad-6359dc02c39a', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '3b29fcfb-5687-47bb-85ad-6359dc02c39a', 'distance_longer_than',         6.0,   '{}'),
    (gen_random_uuid(), '3b29fcfb-5687-47bb-85ad-6359dc02c39a', 'pace_faster_than',             330.0, '{}'),

    -- W4: Distance Ramp 7K
    (gen_random_uuid(), '60550fef-4588-4bcc-8193-1fe7fb8dc2b4', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '60550fef-4588-4bcc-8193-1fe7fb8dc2b4', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '60550fef-4588-4bcc-8193-1fe7fb8dc2b4', 'distance_longer_than',         7.0,   '{}'),
    (gen_random_uuid(), '60550fef-4588-4bcc-8193-1fe7fb8dc2b4', 'pace_faster_than',             330.0, '{}'),

    -- W5: Short Super-Fast 4K
    (gen_random_uuid(), '131ea925-6c02-473b-9e45-df34e2593555', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '131ea925-6c02-473b-9e45-df34e2593555', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '131ea925-6c02-473b-9e45-df34e2593555', 'distance_longer_than',         4.0,   '{}'),
    (gen_random_uuid(), '131ea925-6c02-473b-9e45-df34e2593555', 'pace_faster_than',             300.0, '{}'),

    -- W6: Distance Ramp 8K
    (gen_random_uuid(), '26c01351-33a6-43b8-855e-889f93cc0d29', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '26c01351-33a6-43b8-855e-889f93cc0d29', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '26c01351-33a6-43b8-855e-889f93cc0d29', 'distance_longer_than',         8.0,   '{}'),
    (gen_random_uuid(), '26c01351-33a6-43b8-855e-889f93cc0d29', 'pace_faster_than',             330.0, '{}'),

    -- W7: Distance Ramp 9K
    (gen_random_uuid(), '2c3789e2-a7f7-4a2a-8ea9-43a4d6112e4d', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '2c3789e2-a7f7-4a2a-8ea9-43a4d6112e4d', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '2c3789e2-a7f7-4a2a-8ea9-43a4d6112e4d', 'distance_longer_than',         9.0,   '{}'),
    (gen_random_uuid(), '2c3789e2-a7f7-4a2a-8ea9-43a4d6112e4d', 'pace_faster_than',             330.0, '{}'),

    -- W8: Short Super-Fast 5K
    (gen_random_uuid(), 'c4666951-77f8-4c0c-9d49-3fd8ea26441f', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'c4666951-77f8-4c0c-9d49-3fd8ea26441f', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), 'c4666951-77f8-4c0c-9d49-3fd8ea26441f', 'distance_longer_than',         5.0,   '{}'),
    (gen_random_uuid(), 'c4666951-77f8-4c0c-9d49-3fd8ea26441f', 'pace_faster_than',             300.0, '{}'),

    -- W9: Distance Ramp 10K
    (gen_random_uuid(), 'f543902a-6ec2-4d24-bae3-0990425f2c3b', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'f543902a-6ec2-4d24-bae3-0990425f2c3b', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), 'f543902a-6ec2-4d24-bae3-0990425f2c3b', 'distance_longer_than',         10.0,  '{}'),
    (gen_random_uuid(), 'f543902a-6ec2-4d24-bae3-0990425f2c3b', 'pace_faster_than',             330.0, '{}'),

    -- W10: Distance Ramp 11K
    (gen_random_uuid(), '7d8b219c-6e67-4bca-a5b6-aac9438fcf97', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '7d8b219c-6e67-4bca-a5b6-aac9438fcf97', 'days_after_previous_workout',  1.0,   '{}'),
    (gen_random_uuid(), '7d8b219c-6e67-4bca-a5b6-aac9438fcf97', 'distance_longer_than',         11.0,  '{}'),
    (gen_random_uuid(), '7d8b219c-6e67-4bca-a5b6-aac9438fcf97', 'pace_faster_than',             330.0, '{}'),

    -- W11: Final Distance Test 12K
    (gen_random_uuid(), '55d617e3-aef9-4947-ab91-e9092a9a7594', 'activity_type_is',             NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '55d617e3-aef9-4947-ab91-e9092a9a7594', 'days_after_previous_workout',  2.0,   '{}'),
    (gen_random_uuid(), '55d617e3-aef9-4947-ab91-e9092a9a7594', 'distance_longer_than',         12.0,  '{}'),
    (gen_random_uuid(), '55d617e3-aef9-4947-ab91-e9092a9a7594', 'pace_faster_than',             330.0, '{}')
;
