-- Insert test user
INSERT INTO auth.user (id, name, email, "emailVerified", "updatedAt")
VALUES ('00000000-0000-0000-0000-000000000000', 'testuser', 'testuser@email.com', false, CURRENT_TIMESTAMP);

-- Insert test user credentials
INSERT INTO auth."account" (id, "accountId", "providerId", "userId", password, "updatedAt")
VALUES
(
    '00000000-0000-0000-0000-000000000000',
    '00000000-0000-0000-0000-000000000000',
    'credential',
    '00000000-0000-0000-0000-000000000000',
    '93a192bfcac08a0e3b0218712d0a5a34:b9de26c75d50938e5c0492c5d38e1a9bbf81cbcd08e4c04405c2a06dea4d999ebfd60ce1bb5ae9f5b7e0dc7472d143267fde371605cfade7bd96222d2fc5b02d',
    CURRENT_TIMESTAMP
);

-- Add test tasks
INSERT INTO app.tasks
(
    title, created_by,
    notes,
    deleted_at, created_at, updated_at
)
VALUES
(
    'Test Task 1',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    NULL,
    '2025-06-01T18:00:00.000Z',
    '2025-06-05T18:00:00.000Z'
),
(
    'Test Task 2',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    NULL,
    '2025-06-02T18:00:00.000Z',
    '2025-06-04T18:00:00.000Z'
),
(
    'Test Task 3',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    NULL,
    '2025-06-03T18:00:00.000Z',
    '2025-06-03T18:00:00.000Z'
),
(
    'Test Task 4',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    NULL,
    '2025-06-04T18:00:00.000Z',
    '2025-06-02T18:00:00.000Z'
),
(
    'Test Task 5',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    NULL,
    '2025-06-05T18:00:00.000Z',
    '2025-06-01T18:00:00.000Z'
),
(
    'Test Task 6',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    '2025-06-05T18:00:00.000Z',
    '2025-06-01T18:00:00.000Z',
    '2025-06-05T18:00:00.000Z'
),
(
    'Test Task 7',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    '2025-06-04T18:00:00.000Z',
    '2025-06-02T18:00:00.000Z',
    '2025-06-04T18:00:00.000Z'
),
(
    'Test Task 8',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    '2025-06-03T18:00:00.000Z',
    '2025-06-03T18:00:00.000Z',
    '2025-06-03T18:00:00.000Z'
),
(
    'Test Task 9',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    '2025-06-02T18:00:00.000Z',
    '2025-06-04T18:00:00.000Z',
    '2025-06-02T18:00:00.000Z'
),
(
    'Test Task 10',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for a base query',
    '2025-06-01T18:00:00.000Z',
    '2025-06-05T18:00:00.000Z',
    '2025-06-01T18:00:00.000Z'
);

-- Add test tasks for only start time
INSERT INTO app.tasks
(
    title, created_by,
    notes, start_on, start_at,
    created_at, updated_at
)
VALUES
(
    'Test Task S1',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    '2025-01-01',
    NULL,
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S2',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    NULL,
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S3',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    '2025-01-02',
    NULL,
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S4',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    NULL,
    '2025-01-02T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S5',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    '2025-01-03',
    NULL,
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S6',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    NULL,
    '2025-01-03T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S7',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    '2025-01-04',
    NULL,
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task S8',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for start filter queries',
    NULL,
    '2025-01-04T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
);

-- Add testing tasks for only deadline time
INSERT INTO app.tasks
(
    title, created_by,
    notes, deadline,
    created_at, updated_at
)
VALUES
(
    'Test Task D1',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-01',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D2',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-02',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D3',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-03',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D4',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-04',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D5',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-05',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D6',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-06',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D7',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-07',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
),
(
    'Test Task D8',
    '00000000-0000-0000-0000-000000000000',
    'This is a test for deadline filter queries',
    '2025-01-08',
    '2025-01-01T18:00:00.000Z',
    '2025-01-01T18:00:00.000Z'
);

DO $$
DECLARE
    dummy_user UUID := '00000000-0000-0000-0000-000000000000';
    i INT;
BEGIN
    FOR i IN 1..25 LOOP
        INSERT INTO app.tasks (
            title, notes, start_on, start_at, deadline,
            deleted_at, created_at, updated_at, created_by
        ) VALUES (
            -- Title
            'Test Task ' || i,

            -- Notes
            'Notes for task ' || i,

            -- start_on: populate for i % 3 = 0
            CASE WHEN i % 3 = 0 THEN DATE '2026-03-25' + (i % 5) ELSE NULL END,

            -- start_at: populate for i % 3 = 1
            CASE WHEN i % 3 = 1 THEN TIMESTAMPTZ '2026-03-25 09:00:00+00' + (i::text || ' hours')::interval ELSE NULL END,

            -- deadline: populate for i % 2 = 0
            CASE WHEN i % 2 = 0 THEN DATE '2026-04-01' + (i % 10) ELSE NULL END,

            -- deleted_at: populate for i % 5 = 0
            CASE WHEN i % 5 = 0 THEN TIMESTAMPTZ '2026-03-25 12:00:00+00' + (i::text || ' hours')::interval ELSE NULL END,

            -- created_at: always some base + i hours
            TIMESTAMPTZ '2026-03-20 08:00:00+00' + (i::text || ' hours')::interval,

            -- updated_at: match deleted_at if deleted, else created_at
            CASE WHEN i % 5 = 0 THEN TIMESTAMPTZ '2026-03-25 12:00:00+00' + (i::text || ' hours')::interval
                 ELSE TIMESTAMPTZ '2026-03-20 08:00:00+00' + (i::text || ' hours')::interval END,

            -- created_by
            dummy_user
        );
    END LOOP;
END $$;

-- Add testings tags
INSERT INTO app.tags (
    id, label, created_by,
    category, deleted_at,
    created_at, updated_at
)
VALUES
(
    '00000000-0000-0000-0000-000000000001',
    'Test Tag 1',
    '00000000-0000-0000-0000-000000000000',
    NULL,
    NULL,
    '2025-06-01T18:00:00.000Z',
    '2025-06-06T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000002',
    'Test Tag 2',
    '00000000-0000-0000-0000-000000000000',
    NULL,
    NULL,
    '2025-06-02T18:00:00.000Z',
    '2025-06-05T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000003',
    'Test Tag 3',
    '00000000-0000-0000-0000-000000000000',
    NULL,
    NULL,
    '2025-06-03T18:00:00.000Z',
    '2025-06-04T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000004',
    'Test Tag 4',
    '00000000-0000-0000-0000-000000000000',
    'Testing',
    NULL,
    '2025-06-04T18:00:00.000Z',
    '2025-06-03T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000005',
    'Test Tag 5',
    '00000000-0000-0000-0000-000000000000',
    'Testing',
    NULL,
    '2025-06-05T18:00:00.000Z',
    '2025-06-02T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000006',
    'Test Tag 6',
    '00000000-0000-0000-0000-000000000000',
    'Testing',
    NULL,
    '2025-06-06T18:00:00.000Z',
    '2025-06-01T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000007',
    'Test Tag 7',
    '00000000-0000-0000-0000-000000000000',
    'Deleted',
    '2025-03-01T18:00:00.000Z',
    '2025-06-04T18:00:00.000Z',
    '2025-06-03T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000008',
    'Test Tag 8',
    '00000000-0000-0000-0000-000000000000',
    'Deleted',
    '2025-03-01T18:00:00.000Z',
    '2025-06-05T18:00:00.000Z',
    '2025-06-02T18:00:00.000Z'
),
(
    '00000000-0000-0000-0000-000000000009',
    'Test Tag 9',
    '00000000-0000-0000-0000-000000000000',
    'Deleted',
    '2025-03-01T18:00:00.000Z',
    '2025-06-06T18:00:00.000Z',
    '2025-06-01T18:00:00.000Z'
);
