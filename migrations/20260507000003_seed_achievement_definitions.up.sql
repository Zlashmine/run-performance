-- Seed all 30 achievement definitions
INSERT INTO achievement_definitions (slug, name, description, icon, xp_reward, rarity, category, is_secret, sort_order) VALUES
-- Distance (10)
('first_run',         'First Strides',         'Complete your first run',                          'Footprints', 50,  'common',    'distance',    false, 1),
('run_25km',          'Quarter Century',        'Run 25 km total',                                  'MapPin',     50,  'common',    'distance',    false, 2),
('run_100km',         'Century Runner',         'Run 100 km total',                                 'Trophy',     100, 'rare',      'distance',    false, 3),
('run_250km',         'Dedicated Runner',       'Run 250 km total',                                 'Trophy',     100, 'rare',      'distance',    false, 4),
('run_500km',         'Iron Legs',              'Run 500 km total',                                 'Dumbbell',   150, 'epic',      'distance',    false, 5),
('run_1000km',        'Thousand Mile Club',     'Run 1,000 km total',                               'Star',       200, 'legendary', 'distance',    false, 6),
('run_5k_once',       '5K Finisher',            'Complete a run of at least 5 km',                  'Medal',      50,  'common',    'distance',    false, 7),
('run_10k_once',      '10K Finisher',           'Complete a run of at least 10 km',                 'Medal',      50,  'common',    'distance',    false, 8),
('run_half_once',     'Half Marathon',          'Complete a run of at least 21.1 km',               'Award',      100, 'rare',      'distance',    false, 9),
('run_marathon_once', 'Marathoner',             'Complete a run of at least 42.2 km',               'Crown',      150, 'epic',      'distance',    false, 10),
-- Pace (6)
('pace_sub6',         'Sub-6 Pacer',            'Complete a run with avg pace under 6:00/km',       'Zap',        100, 'rare',      'pace',        false, 11),
('pace_sub5',         'Sub-5 Blazer',           'Complete a run with avg pace under 5:00/km',       'Zap',        150, 'epic',      'pace',        false, 12),
('pace_sub430',       'Elite Speedster',        'Complete a run with avg pace under 4:30/km',       'Flame',      200, 'legendary', 'pace',        false, 13),
('consistent_pace',   'Metronome',              '3 runs in a row within 10 sec/km of each other',   'Target',     100, 'rare',      'pace',        false, 14),
('negative_split',    'Negative Splitter',      '5 runs where the second half was faster',          'TrendingUp', 100, 'rare',      'pace',        false, 15),
('personal_best_streak','PR Machine',           'Set 3 personal records',                           'Sparkles',   150, 'epic',      'pace',        false, 16),
-- Streak (5)
('streak_3',          'Hot Streak',             '3-day running streak',                             'Flame',      50,  'common',    'streak',      false, 17),
('streak_7',          'On A Roll',              '7-day running streak',                             'Flame',      100, 'rare',      'streak',      false, 18),
('streak_14',         'Unstoppable',            '14-day running streak',                            'Flame',      150, 'epic',      'streak',      false, 19),
('streak_30',         'Month Warrior',          '30-day running streak',                            'Shield',     200, 'legendary', 'streak',      false, 20),
('comeback',          'Comeback Kid',           'Run after a 30+ day gap',                          'RefreshCw',  100, 'rare',      'streak',      false, 21),
-- Consistency (4)
('runs_50',           'Half Century',           'Complete 50 total runs',                           'ListChecks', 100, 'rare',      'consistency', false, 22),
('runs_100',          'Century Club',           'Complete 100 total runs',                          'ListChecks', 150, 'epic',      'consistency', false, 23),
('runs_365',          'Year of Runs',           'Complete 365 total runs',                          'Calendar',   200, 'legendary', 'consistency', false, 24),
('monday_10',         'Monday Motivation',      'Run on at least 10 Mondays',                       'Calendar',   100, 'rare',      'consistency', false, 25),
-- Secret (5)
('night_owl',         'Night Owl',              'Complete a run after 22:00',                       'Moon',       100, 'rare',      'secret',      true,  26),
('early_bird',        'Early Bird',             'Complete a run before 06:00',                      'Sunrise',    100, 'rare',      'secret',      true,  27),
('new_years_runner',  'New Year''s Runner',      'Run on January 1st',                              'PartyPopper',150, 'epic',      'secret',      true,  28),
('speedy_upload',     'Speed Demon',            'Upload an activity with pace under 3:30/km',       'Gauge',      200, 'legendary', 'secret',      true,  29),
('explorer',          'The Explorer',           'Log runs in 5 different months',                   'Globe',      100, 'rare',      'secret',      true,  30);
