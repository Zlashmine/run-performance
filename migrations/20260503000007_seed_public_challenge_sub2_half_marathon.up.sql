-- ── 0. Ensure system user exists (idempotent) ────────────────────────────────
-- users table: (id UUID PK, google_id TEXT NOT NULL, email TEXT NOT NULL,
--               created_at TIMESTAMP) — no updated_at column.
INSERT INTO users (id, google_id, email, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'system',
    'system@run-performance.internal',
    now()
) ON CONFLICT (id) DO NOTHING;

-- ── 1. Challenge ─────────────────────────────────────────────────────────────
-- started_at is intentionally NULL (evergreen public template).
-- The opt_in_challenge service sets started_at = now() on the clone at opt-in.
INSERT INTO challenges
    (id, user_id, name, description, is_recurring, status, is_public, created_at, updated_at)
VALUES (
    '480cc104-9aef-4c98-b733-94f027ac1b6e',
    '00000000-0000-0000-0000-000000000001',
    'Sub-2 Hour Half Marathon Performance Builder',
    'A high-intensity half-marathon challenge for runners aiming to finish below 2 hours. The program combines easy aerobic running, tempo work, race-pace preparation, and progressive long runs to build endurance and speed.',
    false,
    'active',
    true,
    now(), now()
) ON CONFLICT (id) DO NOTHING;

-- ── 2. Workouts ──────────────────────────────────────────────────────────────
INSERT INTO challenge_workouts
    (id, challenge_id, position, name, description, created_at, updated_at)
VALUES
    ('c75bf6ae-7dde-40df-b3ad-cf3d31ba8367', '480cc104-9aef-4c98-b733-94f027ac1b6e',  1, 'Easy Aerobic Run — Week 1',         'Run 6 km at easy aerobic pace to establish your training base.',                                          now(), now()),
    ('7ef97786-b85f-4bf8-8629-692d9078b561', '480cc104-9aef-4c98-b733-94f027ac1b6e',  2, 'Tempo Run — Week 1',                'Run 5 km at controlled tempo pace to begin building sustained speed.',                                    now(), now()),
    ('402447a7-9ef1-4df0-ba24-02a68b668134', '480cc104-9aef-4c98-b733-94f027ac1b6e',  3, 'Long Run — Week 1',                 'Run 10 km at easy aerobic pace to start developing half-marathon endurance.',                               now(), now()),
    ('50c88e02-3ca9-4131-9d9b-bd380ff98b29', '480cc104-9aef-4c98-b733-94f027ac1b6e',  4, 'Easy Aerobic Run — Week 2',         'Run 7 km at easy aerobic pace to add relaxed volume.',                                                    now(), now()),
    ('1111018c-633e-4f1f-b221-fb911ab0fddc', '480cc104-9aef-4c98-b733-94f027ac1b6e',  5, 'Race Prep Run — Week 2',            'Run 5 km faster than 6:00 per km to introduce sub-2 half-marathon speed.',                                now(), now()),
    ('6c28e6ea-6daa-4517-b734-5e81897dac8a', '480cc104-9aef-4c98-b733-94f027ac1b6e',  6, 'Long Run — Week 2',                 'Run 12 km at easy aerobic pace to extend your endurance range.',                                          now(), now()),
    ('4f937c9f-0e1f-400d-af74-f68a8518c653', '480cc104-9aef-4c98-b733-94f027ac1b6e',  7, 'Easy Aerobic Run — Week 3',         'Run 7 km at easy aerobic pace to recover while maintaining mileage.',                                     now(), now()),
    ('b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', '480cc104-9aef-4c98-b733-94f027ac1b6e',  8, 'Tempo Run — Week 3',                'Run 6 km at controlled tempo pace to improve your ability to hold effort.',                                now(), now()),
    ('9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', '480cc104-9aef-4c98-b733-94f027ac1b6e',  9, 'Long Run — Week 3',                 'Run 14 km at easy aerobic pace to build durable endurance.',                                              now(), now()),
    ('51b11408-76ea-461d-b024-6dc92131c435', '480cc104-9aef-4c98-b733-94f027ac1b6e', 10, 'Easy Aerobic Run — Week 4',         'Run 8 km at easy aerobic pace to strengthen your base before harder work.',                               now(), now()),
    ('55c4add6-6651-4328-a3f4-cd531a112c71', '480cc104-9aef-4c98-b733-94f027ac1b6e', 11, 'Race Prep Run — Week 4',            'Run 6 km faster than 6:00 per km to sharpen your target-race rhythm.',                                   now(), now()),
    ('77ea400a-7ef9-4062-9e51-51b781d3e574', '480cc104-9aef-4c98-b733-94f027ac1b6e', 12, 'Long Run — Week 4',                 'Run 15.5 km at easy aerobic pace to continue progressing your long-run capacity.',                        now(), now()),
    ('53ca9bce-dd39-475c-adde-67eb10bd01a8', '480cc104-9aef-4c98-b733-94f027ac1b6e', 13, 'Easy Aerobic Run — Week 5',         'Run 8 km at easy aerobic pace to support recovery and consistency.',                                     now(), now()),
    ('14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', '480cc104-9aef-4c98-b733-94f027ac1b6e', 14, 'Tempo Run — Week 5',                'Run 7 km at controlled tempo pace to improve stamina under pressure.',                                    now(), now()),
    ('0951204f-082f-4203-a521-5e612b8263ed', '480cc104-9aef-4c98-b733-94f027ac1b6e', 15, 'Long Run — Week 5',                 'Run 17 km at easy aerobic pace to prepare your body for extended time on feet.',                          now(), now()),
    ('fe9f4d77-9e32-4437-adad-51575dfe3031', '480cc104-9aef-4c98-b733-94f027ac1b6e', 16, 'Easy Aerobic Run — Week 6',         'Run 9 km at easy aerobic pace to reinforce your endurance base.',                                        now(), now()),
    ('9002413c-a5b5-4933-80a3-da1027338b1a', '480cc104-9aef-4c98-b733-94f027ac1b6e', 17, 'Race Prep Run — Week 6',            'Run 7 km faster than 6:00 per km to build confidence near race intensity.',                               now(), now()),
    ('bb884792-fb6c-435c-85f9-d24711b0b59a', '480cc104-9aef-4c98-b733-94f027ac1b6e', 18, 'Long Run — Week 6',                 'Run 18.5 km at easy aerobic pace to build strength for the final race distance.',                        now(), now()),
    ('30b269c7-b526-4ff4-bd2c-2187ac219373', '480cc104-9aef-4c98-b733-94f027ac1b6e', 19, 'Easy Aerobic Run — Week 7',         'Run 9 km at easy aerobic pace to absorb the harder training block.',                                     now(), now()),
    ('786c8250-35b1-4782-8c07-f115fdcab411', '480cc104-9aef-4c98-b733-94f027ac1b6e', 20, 'Tempo Run — Week 7',                'Run 8 km at controlled tempo pace to extend your sustained-speed endurance.',                            now(), now()),
    ('f228fbf9-726e-4c4a-b9f0-1df28349c79f', '480cc104-9aef-4c98-b733-94f027ac1b6e', 21, 'Long Run — Week 7',                 'Run 20 km at easy aerobic pace to approach the full half-marathon distance.',                            now(), now()),
    ('6572d71c-fe40-44c5-9df3-44b152f27c32', '480cc104-9aef-4c98-b733-94f027ac1b6e', 22, 'Easy Aerobic Run — Week 8',         'Run 8 km at easy aerobic pace to recover from the peak long run.',                                       now(), now()),
    ('b17f22a2-96ab-427b-bdeb-19f67e22c2dd', '480cc104-9aef-4c98-b733-94f027ac1b6e', 23, 'Race Prep Run — Week 8',            'Run 8 km faster than 6:00 per km to practise strong race-focused pacing.',                               now(), now()),
    ('705eb493-38a1-49a6-8c65-c7c88f3a4a5d', '480cc104-9aef-4c98-b733-94f027ac1b6e', 24, 'Long Run — Week 8',                 'Run 18 km at easy aerobic pace to maintain endurance without overreaching.',                             now(), now()),
    ('b1ec42fe-e3bb-43d8-a709-2f5d12362852', '480cc104-9aef-4c98-b733-94f027ac1b6e', 25, 'Easy Aerobic Run — Week 9',         'Run 8 km at easy aerobic pace to keep volume steady during sharpening work.',                            now(), now()),
    ('a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', '480cc104-9aef-4c98-b733-94f027ac1b6e', 26, 'Tempo Run — Week 9',                'Run 9 km at controlled tempo pace to develop late-race strength.',                                       now(), now()),
    ('aafd044e-adf7-4931-bab6-8f976e3d1122', '480cc104-9aef-4c98-b733-94f027ac1b6e', 27, 'Long Run — Week 9',                 'Run 21 km at easy aerobic pace to prove you can cover the race distance.',                                now(), now()),
    ('fcb49daa-7b00-4714-b1de-36b6b9c2cc5a', '480cc104-9aef-4c98-b733-94f027ac1b6e', 28, 'Easy Aerobic Run — Week 10',        'Run 7 km at easy aerobic pace to begin freshening up while staying active.',                             now(), now()),
    ('1309af4f-bcbd-4ec7-9365-3134a7e489a3', '480cc104-9aef-4c98-b733-94f027ac1b6e', 29, 'Race Prep Run — Week 10',           'Run 9 km faster than 6:00 per km to lock in efficient target-race movement.',                            now(), now()),
    ('d8b139b5-1aad-417d-ac96-7fe8120d93f4', '480cc104-9aef-4c98-b733-94f027ac1b6e', 30, 'Long Run — Week 10',                'Run 16 km at easy aerobic pace to retain endurance while reducing fatigue.',                             now(), now()),
    ('9ba6be38-e91a-439c-b2aa-474ddc753d26', '480cc104-9aef-4c98-b733-94f027ac1b6e', 31, 'Easy Aerobic Run — Week 11',        'Run 6 km at easy aerobic pace to recover while keeping your legs moving.',                               now(), now()),
    ('8768a227-ea92-4534-b5ff-4eb0a292af88', '480cc104-9aef-4c98-b733-94f027ac1b6e', 32, 'Tempo Run — Week 11',               'Run 6 km at controlled tempo pace to sharpen without adding too much fatigue.',                         now(), now()),
    ('7ad50788-c4cf-4ced-a78b-e9570b2001ca', '480cc104-9aef-4c98-b733-94f027ac1b6e', 33, 'Long Run — Week 11',                'Run 12 km at easy aerobic pace to maintain endurance during the taper.',                                  now(), now()),
    ('9a8c3515-b853-41d3-84ac-80102b712656', '480cc104-9aef-4c98-b733-94f027ac1b6e', 34, 'Easy Shakeout Run — Week 12',       'Run 5 km at easy aerobic pace to stay loose before the final effort.',                                   now(), now()),
    ('52f0525f-dd49-4090-850b-c5502fc19a38', '480cc104-9aef-4c98-b733-94f027ac1b6e', 35, 'Final Race Prep Run — Week 12',     'Run 5 km faster than 6:00 per km to activate your race pace without overloading.',                       now(), now()),
    ('eeedd461-b426-4949-8f29-29f3ca2d4e6a', '480cc104-9aef-4c98-b733-94f027ac1b6e', 36, 'Sub-2 Half Marathon Attempt — Week 12', 'Run the half-marathon distance faster than 5:41 per km to finish below 2 hours.',                  now(), now())
ON CONFLICT (id) DO NOTHING;

-- ── 3. Requirements ──────────────────────────────────────────────────────────
-- Requirements use gen_random_uuid() — idempotency is guaranteed by sqlx
-- migration tracking (this file runs exactly once).
-- NULL value is used for requirement types that take no numeric threshold.
INSERT INTO challenge_workout_requirements
    (id, challenge_workout_id, requirement_type, value, params)
VALUES
    -- W1: Easy Aerobic Run — Week 1
    (gen_random_uuid(), 'c75bf6ae-7dde-40df-b3ad-cf3d31ba8367', 'activity_type_is',     NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'c75bf6ae-7dde-40df-b3ad-cf3d31ba8367', 'distance_longer_than', 6.0,   '{}'),
    (gen_random_uuid(), 'c75bf6ae-7dde-40df-b3ad-cf3d31ba8367', 'pace_slower_than',     450.0, '{}'),

    -- W2: Tempo Run — Week 1
    (gen_random_uuid(), '7ef97786-b85f-4bf8-8629-692d9078b561', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '7ef97786-b85f-4bf8-8629-692d9078b561', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '7ef97786-b85f-4bf8-8629-692d9078b561', 'distance_longer_than',       5.0,  '{}'),
    (gen_random_uuid(), '7ef97786-b85f-4bf8-8629-692d9078b561', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), '7ef97786-b85f-4bf8-8629-692d9078b561', 'pace_slower_than',           390.0,'{}'),

    -- W3: Long Run — Week 1
    (gen_random_uuid(), '402447a7-9ef1-4df0-ba24-02a68b668134', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '402447a7-9ef1-4df0-ba24-02a68b668134', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '402447a7-9ef1-4df0-ba24-02a68b668134', 'distance_longer_than',          10.0,  '{}'),
    (gen_random_uuid(), '402447a7-9ef1-4df0-ba24-02a68b668134', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '402447a7-9ef1-4df0-ba24-02a68b668134', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W4: Easy Aerobic Run — Week 2
    (gen_random_uuid(), '50c88e02-3ca9-4131-9d9b-bd380ff98b29', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '50c88e02-3ca9-4131-9d9b-bd380ff98b29', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '50c88e02-3ca9-4131-9d9b-bd380ff98b29', 'distance_longer_than',       7.0,  '{}'),
    (gen_random_uuid(), '50c88e02-3ca9-4131-9d9b-bd380ff98b29', 'pace_slower_than',           450.0,'{}'),

    -- W5: Race Prep Run — Week 2
    (gen_random_uuid(), '1111018c-633e-4f1f-b221-fb911ab0fddc', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '1111018c-633e-4f1f-b221-fb911ab0fddc', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '1111018c-633e-4f1f-b221-fb911ab0fddc', 'distance_longer_than',       5.0,  '{}'),
    (gen_random_uuid(), '1111018c-633e-4f1f-b221-fb911ab0fddc', 'pace_faster_than',           360.0,'{}'),

    -- W6: Long Run — Week 2
    (gen_random_uuid(), '6c28e6ea-6daa-4517-b734-5e81897dac8a', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '6c28e6ea-6daa-4517-b734-5e81897dac8a', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '6c28e6ea-6daa-4517-b734-5e81897dac8a', 'distance_longer_than',          12.0,  '{}'),
    (gen_random_uuid(), '6c28e6ea-6daa-4517-b734-5e81897dac8a', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '6c28e6ea-6daa-4517-b734-5e81897dac8a', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W7: Easy Aerobic Run — Week 3
    (gen_random_uuid(), '4f937c9f-0e1f-400d-af74-f68a8518c653', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '4f937c9f-0e1f-400d-af74-f68a8518c653', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '4f937c9f-0e1f-400d-af74-f68a8518c653', 'distance_longer_than',       7.0,  '{}'),
    (gen_random_uuid(), '4f937c9f-0e1f-400d-af74-f68a8518c653', 'pace_slower_than',           450.0,'{}'),

    -- W8: Tempo Run — Week 3
    (gen_random_uuid(), 'b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', 'distance_longer_than',       6.0,  '{}'),
    (gen_random_uuid(), 'b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), 'b4087ea7-c5cc-444a-91bd-3be7a2f70fa0', 'pace_slower_than',           390.0,'{}'),

    -- W9: Long Run — Week 3
    (gen_random_uuid(), '9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', 'distance_longer_than',          14.0,  '{}'),
    (gen_random_uuid(), '9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '9c7cc8dd-c235-4bcf-90ec-d56b18abcd03', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W10: Easy Aerobic Run — Week 4
    (gen_random_uuid(), '51b11408-76ea-461d-b024-6dc92131c435', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '51b11408-76ea-461d-b024-6dc92131c435', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '51b11408-76ea-461d-b024-6dc92131c435', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), '51b11408-76ea-461d-b024-6dc92131c435', 'pace_slower_than',           450.0,'{}'),

    -- W11: Race Prep Run — Week 4
    (gen_random_uuid(), '55c4add6-6651-4328-a3f4-cd531a112c71', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '55c4add6-6651-4328-a3f4-cd531a112c71', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '55c4add6-6651-4328-a3f4-cd531a112c71', 'distance_longer_than',       6.0,  '{}'),
    (gen_random_uuid(), '55c4add6-6651-4328-a3f4-cd531a112c71', 'pace_faster_than',           360.0,'{}'),

    -- W12: Long Run — Week 4
    (gen_random_uuid(), '77ea400a-7ef9-4062-9e51-51b781d3e574', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '77ea400a-7ef9-4062-9e51-51b781d3e574', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '77ea400a-7ef9-4062-9e51-51b781d3e574', 'distance_longer_than',          15.5,  '{}'),
    (gen_random_uuid(), '77ea400a-7ef9-4062-9e51-51b781d3e574', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '77ea400a-7ef9-4062-9e51-51b781d3e574', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W13: Easy Aerobic Run — Week 5
    (gen_random_uuid(), '53ca9bce-dd39-475c-adde-67eb10bd01a8', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '53ca9bce-dd39-475c-adde-67eb10bd01a8', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '53ca9bce-dd39-475c-adde-67eb10bd01a8', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), '53ca9bce-dd39-475c-adde-67eb10bd01a8', 'pace_slower_than',           450.0,'{}'),

    -- W14: Tempo Run — Week 5
    (gen_random_uuid(), '14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', 'distance_longer_than',       7.0,  '{}'),
    (gen_random_uuid(), '14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), '14ec25d3-c78a-4fbf-a19e-ac5ece6c45cc', 'pace_slower_than',           390.0,'{}'),

    -- W15: Long Run — Week 5
    (gen_random_uuid(), '0951204f-082f-4203-a521-5e612b8263ed', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '0951204f-082f-4203-a521-5e612b8263ed', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '0951204f-082f-4203-a521-5e612b8263ed', 'distance_longer_than',          17.0,  '{}'),
    (gen_random_uuid(), '0951204f-082f-4203-a521-5e612b8263ed', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '0951204f-082f-4203-a521-5e612b8263ed', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W16: Easy Aerobic Run — Week 6
    (gen_random_uuid(), 'fe9f4d77-9e32-4437-adad-51575dfe3031', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'fe9f4d77-9e32-4437-adad-51575dfe3031', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'fe9f4d77-9e32-4437-adad-51575dfe3031', 'distance_longer_than',       9.0,  '{}'),
    (gen_random_uuid(), 'fe9f4d77-9e32-4437-adad-51575dfe3031', 'pace_slower_than',           450.0,'{}'),

    -- W17: Race Prep Run — Week 6
    (gen_random_uuid(), '9002413c-a5b5-4933-80a3-da1027338b1a', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '9002413c-a5b5-4933-80a3-da1027338b1a', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '9002413c-a5b5-4933-80a3-da1027338b1a', 'distance_longer_than',       7.0,  '{}'),
    (gen_random_uuid(), '9002413c-a5b5-4933-80a3-da1027338b1a', 'pace_faster_than',           360.0,'{}'),

    -- W18: Long Run — Week 6
    (gen_random_uuid(), 'bb884792-fb6c-435c-85f9-d24711b0b59a', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'bb884792-fb6c-435c-85f9-d24711b0b59a', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), 'bb884792-fb6c-435c-85f9-d24711b0b59a', 'distance_longer_than',          18.5,  '{}'),
    (gen_random_uuid(), 'bb884792-fb6c-435c-85f9-d24711b0b59a', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), 'bb884792-fb6c-435c-85f9-d24711b0b59a', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W19: Easy Aerobic Run — Week 7
    (gen_random_uuid(), '30b269c7-b526-4ff4-bd2c-2187ac219373', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '30b269c7-b526-4ff4-bd2c-2187ac219373', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '30b269c7-b526-4ff4-bd2c-2187ac219373', 'distance_longer_than',       9.0,  '{}'),
    (gen_random_uuid(), '30b269c7-b526-4ff4-bd2c-2187ac219373', 'pace_slower_than',           450.0,'{}'),

    -- W20: Tempo Run — Week 7
    (gen_random_uuid(), '786c8250-35b1-4782-8c07-f115fdcab411', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '786c8250-35b1-4782-8c07-f115fdcab411', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '786c8250-35b1-4782-8c07-f115fdcab411', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), '786c8250-35b1-4782-8c07-f115fdcab411', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), '786c8250-35b1-4782-8c07-f115fdcab411', 'pace_slower_than',           390.0,'{}'),

    -- W21: Long Run — Week 7
    (gen_random_uuid(), 'f228fbf9-726e-4c4a-b9f0-1df28349c79f', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'f228fbf9-726e-4c4a-b9f0-1df28349c79f', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), 'f228fbf9-726e-4c4a-b9f0-1df28349c79f', 'distance_longer_than',          20.0,  '{}'),
    (gen_random_uuid(), 'f228fbf9-726e-4c4a-b9f0-1df28349c79f', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), 'f228fbf9-726e-4c4a-b9f0-1df28349c79f', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W22: Easy Aerobic Run — Week 8
    (gen_random_uuid(), '6572d71c-fe40-44c5-9df3-44b152f27c32', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '6572d71c-fe40-44c5-9df3-44b152f27c32', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '6572d71c-fe40-44c5-9df3-44b152f27c32', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), '6572d71c-fe40-44c5-9df3-44b152f27c32', 'pace_slower_than',           450.0,'{}'),

    -- W23: Race Prep Run — Week 8
    (gen_random_uuid(), 'b17f22a2-96ab-427b-bdeb-19f67e22c2dd', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'b17f22a2-96ab-427b-bdeb-19f67e22c2dd', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'b17f22a2-96ab-427b-bdeb-19f67e22c2dd', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), 'b17f22a2-96ab-427b-bdeb-19f67e22c2dd', 'pace_faster_than',           360.0,'{}'),

    -- W24: Long Run — Week 8
    (gen_random_uuid(), '705eb493-38a1-49a6-8c65-c7c88f3a4a5d', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '705eb493-38a1-49a6-8c65-c7c88f3a4a5d', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '705eb493-38a1-49a6-8c65-c7c88f3a4a5d', 'distance_longer_than',          18.0,  '{}'),
    (gen_random_uuid(), '705eb493-38a1-49a6-8c65-c7c88f3a4a5d', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '705eb493-38a1-49a6-8c65-c7c88f3a4a5d', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W25: Easy Aerobic Run — Week 9
    (gen_random_uuid(), 'b1ec42fe-e3bb-43d8-a709-2f5d12362852', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'b1ec42fe-e3bb-43d8-a709-2f5d12362852', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'b1ec42fe-e3bb-43d8-a709-2f5d12362852', 'distance_longer_than',       8.0,  '{}'),
    (gen_random_uuid(), 'b1ec42fe-e3bb-43d8-a709-2f5d12362852', 'pace_slower_than',           450.0,'{}'),

    -- W26: Tempo Run — Week 9
    (gen_random_uuid(), 'a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', 'distance_longer_than',       9.0,  '{}'),
    (gen_random_uuid(), 'a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), 'a8cc54cb-9d5f-4ec2-8734-d75dfe1900e2', 'pace_slower_than',           390.0,'{}'),

    -- W27: Long Run — Week 9
    (gen_random_uuid(), 'aafd044e-adf7-4931-bab6-8f976e3d1122', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'aafd044e-adf7-4931-bab6-8f976e3d1122', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), 'aafd044e-adf7-4931-bab6-8f976e3d1122', 'distance_longer_than',          21.0,  '{}'),
    (gen_random_uuid(), 'aafd044e-adf7-4931-bab6-8f976e3d1122', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), 'aafd044e-adf7-4931-bab6-8f976e3d1122', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W28: Easy Aerobic Run — Week 10
    (gen_random_uuid(), 'fcb49daa-7b00-4714-b1de-36b6b9c2cc5a', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'fcb49daa-7b00-4714-b1de-36b6b9c2cc5a', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), 'fcb49daa-7b00-4714-b1de-36b6b9c2cc5a', 'distance_longer_than',       7.0,  '{}'),
    (gen_random_uuid(), 'fcb49daa-7b00-4714-b1de-36b6b9c2cc5a', 'pace_slower_than',           450.0,'{}'),

    -- W29: Race Prep Run — Week 10
    (gen_random_uuid(), '1309af4f-bcbd-4ec7-9365-3134a7e489a3', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '1309af4f-bcbd-4ec7-9365-3134a7e489a3', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '1309af4f-bcbd-4ec7-9365-3134a7e489a3', 'distance_longer_than',       9.0,  '{}'),
    (gen_random_uuid(), '1309af4f-bcbd-4ec7-9365-3134a7e489a3', 'pace_faster_than',           360.0,'{}'),

    -- W30: Long Run — Week 10
    (gen_random_uuid(), 'd8b139b5-1aad-417d-ac96-7fe8120d93f4', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'd8b139b5-1aad-417d-ac96-7fe8120d93f4', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), 'd8b139b5-1aad-417d-ac96-7fe8120d93f4', 'distance_longer_than',          16.0,  '{}'),
    (gen_random_uuid(), 'd8b139b5-1aad-417d-ac96-7fe8120d93f4', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), 'd8b139b5-1aad-417d-ac96-7fe8120d93f4', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W31: Easy Aerobic Run — Week 11
    (gen_random_uuid(), '9ba6be38-e91a-439c-b2aa-474ddc753d26', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '9ba6be38-e91a-439c-b2aa-474ddc753d26', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '9ba6be38-e91a-439c-b2aa-474ddc753d26', 'distance_longer_than',       6.0,  '{}'),
    (gen_random_uuid(), '9ba6be38-e91a-439c-b2aa-474ddc753d26', 'pace_slower_than',           450.0,'{}'),

    -- W32: Tempo Run — Week 11
    (gen_random_uuid(), '8768a227-ea92-4534-b5ff-4eb0a292af88', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '8768a227-ea92-4534-b5ff-4eb0a292af88', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '8768a227-ea92-4534-b5ff-4eb0a292af88', 'distance_longer_than',       6.0,  '{}'),
    (gen_random_uuid(), '8768a227-ea92-4534-b5ff-4eb0a292af88', 'pace_faster_than',           420.0,'{}'),
    (gen_random_uuid(), '8768a227-ea92-4534-b5ff-4eb0a292af88', 'pace_slower_than',           390.0,'{}'),

    -- W33: Long Run — Week 11
    (gen_random_uuid(), '7ad50788-c4cf-4ced-a78b-e9570b2001ca', 'activity_type_is',              NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '7ad50788-c4cf-4ced-a78b-e9570b2001ca', 'days_after_previous_workout',   2.0,   '{}'),
    (gen_random_uuid(), '7ad50788-c4cf-4ced-a78b-e9570b2001ca', 'distance_longer_than',          12.0,  '{}'),
    (gen_random_uuid(), '7ad50788-c4cf-4ced-a78b-e9570b2001ca', 'pace_slower_than',              450.0, '{}'),
    (gen_random_uuid(), '7ad50788-c4cf-4ced-a78b-e9570b2001ca', 'distance_increased_by_percent', 10.0,  '{}'),

    -- W34: Easy Shakeout Run — Week 12
    (gen_random_uuid(), '9a8c3515-b853-41d3-84ac-80102b712656', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '9a8c3515-b853-41d3-84ac-80102b712656', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '9a8c3515-b853-41d3-84ac-80102b712656', 'distance_longer_than',       5.0,  '{}'),
    (gen_random_uuid(), '9a8c3515-b853-41d3-84ac-80102b712656', 'pace_slower_than',           450.0,'{}'),

    -- W35: Final Race Prep Run — Week 12
    (gen_random_uuid(), '52f0525f-dd49-4090-850b-c5502fc19a38', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), '52f0525f-dd49-4090-850b-c5502fc19a38', 'days_after_previous_workout', 1.0, '{}'),
    (gen_random_uuid(), '52f0525f-dd49-4090-850b-c5502fc19a38', 'distance_longer_than',       5.0,  '{}'),
    (gen_random_uuid(), '52f0525f-dd49-4090-850b-c5502fc19a38', 'pace_faster_than',           360.0,'{}'),

    -- W36: Sub-2 Half Marathon Attempt — Week 12
    (gen_random_uuid(), 'eeedd461-b426-4949-8f29-29f3ca2d4e6a', 'activity_type_is',          NULL,  '{"activity_type":"Running"}'),
    (gen_random_uuid(), 'eeedd461-b426-4949-8f29-29f3ca2d4e6a', 'days_after_previous_workout', 2.0, '{}'),
    (gen_random_uuid(), 'eeedd461-b426-4949-8f29-29f3ca2d4e6a', 'distance_longer_than',       21.1, '{}'),
    (gen_random_uuid(), 'eeedd461-b426-4949-8f29-29f3ca2d4e6a', 'pace_faster_than',           341.0,'{}')
;
