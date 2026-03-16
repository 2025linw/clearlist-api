-- Insert test user
INSERT INTO auth.user (id, name, email, "emailVerified", "updatedAt")
VALUES ('00000000-0000-0000-0000-000000000001', 'testuser', 'testuser@email.com', false, CURRENT_TIMESTAMP);

-- Insert test user credentials
INSERT INTO auth."account" (id, "accountId", "providerId", "userId", password, "updatedAt")
VALUES
(
    '00000000-0000-0000-0000-000000000001',
    '00000000-0000-0000-0000-000000000001',
    'credential',
    '00000000-0000-0000-0000-000000000001',
    '93a192bfcac08a0e3b0218712d0a5a34:b9de26c75d50938e5c0492c5d38e1a9bbf81cbcd08e4c04405c2a06dea4d999ebfd60ce1bb5ae9f5b7e0dc7472d143267fde371605cfade7bd96222d2fc5b02d',
    CURRENT_TIMESTAMP
);

-- Add test tasks
INSERT INTO app.tasks (id, title, created_by)
VALUES
(
    '00000000-0000-0000-0000-000000000001',
    'Test Task',
    '00000000-0000-0000-0000-000000000001'
);

-- Add test tags
INSERT INTO app.tags (id, label, category, created_by)
VALUES
(
    '00000000-0000-0000-0000-000000000001',
    'Test Tag 1',
    'Testing',
    '00000000-0000-0000-0000-000000000001'
),
(
    '00000000-0000-0000-0000-000000000002',
    'Test Tag 2',
    'Testing',
    '00000000-0000-0000-0000-000000000001'
),
(
    '00000000-0000-0000-0000-000000000003',
    'Test Tag 3',
    'Testing',
    '00000000-0000-0000-0000-000000000001'
);
