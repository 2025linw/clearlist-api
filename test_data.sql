-- Clear tables
TRUNCATE
	auth.users,
	data.tasks,
	data.projects,
	data.areas,
	data.tags
CASCADE;


/* UUID v4 Format
* xxxxxxxx-xxxx-4xTT-9xVV-############
* T: type
* - a: task
* - b: project
* - c: area
* - d: tag
* - f: user
* V: variant
* #: number
*/

/* UUID Details
* Task 0 [1] (00000000-0000-40aa-9000-000000000000): No Title
* Task A [6] (00000000-0000-40aa-9001-############): Date Test
* Task B [8] (00000000-0000-40aa-9002-############): Complete, Log, Trash

* Project 0 [1] (00000000-0000-40bb-9000-000000000000): No Title
* Project A [6] (00000000-0000-40bb-9001-############): Date Test
* Project B [8] (00000000-0000-40bb-9002-############): Complete, Log, Trash
* Project C [1] (00000000-0000-40bb-9003-000000000000): Has Tasks; Not in Area
*   Task C [5] (00000000-0000-40aa-9003-############): In Project C

* Area 0 [1] (00000000-0000-40cc-9000-000000000000): No Title
* Area D [1] (00000000-0000-40cc-9004-000000000000): Has Project (with Tasks)
*   Project D [1] (00000000-0000-40bb-9004-000000000000): In Area D
*     Task D [5] (00000000-0000-40aa-9004-############): In Project D
* Area E [1] (00000000-0000-40cc-9005-000000000000): Has Projects and Tasks
*   Task E [5] (00000000-0000-40aa-9005-############): In Area E
*   Project E [5] (00000000-0000-40bb-9005-############): In Area E; No Tasks
*   Project F [1] (00000000-0000-40bb-9006-000000000000): In Area E; Has Tasks
*     Task F [5] (00000000-0000-40aa-9006-############): In Project F

* Tag 0 [2] (00000000-0000-40dd-9000-############): General Tags
* Tag A [3] (00000000-0000-40dd-9001-############): Priority Tags
*   Task G [3] (00000000-0000-40aa-9007-############): Priority Tag
*   Project G [3] (00000000-0000-40bb-9007-############): Priority Tag
* Tag B [4] (00000000-0000-40dd-9002-############): Scrum Tags
*   Task I [4] (00000000-0000-40aa-9009-############): Scrum Tag
* Tag C [5] (00000000-0000-40dd-9003-############): School Tags
*   Task H [5] (00000000-0000-40aa-9008-############): School Tag
*   Project H [5] (00000000-0000-40bb-9008-############): School Tag

* Users: 00000000-0000-40ff-9000-############: Test Users
*/


/* USERS */
INSERT INTO auth.users
(
    user_id,
	email,
    password_hash
)
VALUES
(
    '00000000-0000-40ff-9000-000000000001',
    'testuser1@email.com',
    'not real hash'
),
(
    '00000000-0000-40ff-9000-000000000002',
    'testuser2@email.com',
    'not real hash'
),
(
    '00000000-0000-40ff-9000-000000000003',
    'testuser3@email.com',
    'not real hash'
);


/* TASKS */
INSERT INTO data.tasks -- Task 0
(
    task_id,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9000-000000000000',
	'No Title',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.tasks -- Task A
(
    task_id,
    task_title,
    notes,

    start_date,
    start_time,
    deadline,

    user_id
)
VALUES
(
    '00000000-0000-40aa-9001-000000000001',
    'Task A1',
    'No start date, No start time, No deadline',

    NULL,
    NULL,
    NULL,

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9001-000000000002',
    'Task A2',
    'Start date, No start time, No deadline',

    '2026-01-01',
    NULL,
    NULL,

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9001-000000000003',
    'Task A3',
    'Start date, Start time, No deadline',

    '2026-01-01',
    '12:00:00',
    NULL,

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9001-000000000004',
    'Task A4',
    'No start date, No start time, Deadline',

    NULL,
    NULL,
    '2026-02-01',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9001-000000000005',
    'Task A5',
    'Start date, No start time, Deadline',

    '2026-01-01',
    NULL,
    '2026-02-01',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9001-000000000006',
    'Task A6',
    'Start date, Start time, Deadline',

    '2026-01-01',
    '12:00:00',
    '2026-02-01',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.tasks -- Task B
(
    task_id,
	task_title,
	notes,

	user_id,
	completed_on,
	logged_on,
	trashed_on
)
VALUES
(
    '00000000-0000-40aa-9002-000000000001',
    'Task B1',
    'No Completed, No Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
	NULL,
	NULL
),
(
    '00000000-0000-40aa-9002-000000000002',
    'Task B2',
    'No Completed, No Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    NULL,
    '2025-01-03'
),
(
    '00000000-0000-40aa-9002-000000000003',
    'Task B3',
    'No Completed, Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    '2025-01-02',
    NULL
),
(
    '00000000-0000-40aa-9002-000000000004',
    'Task B4',
    'No Completed, Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    '2025-01-02',
    '2025-01-03'
),
(
    '00000000-0000-40aa-9002-000000000005',
    'Task B5',
    'Completed, No Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
	NULL,
	NULL
),
(
    '00000000-0000-40aa-9002-000000000006',
    'Task B6',
    'Completed, No Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
	NULL,
    '2025-01-03'
),
(
    '00000000-0000-40aa-9002-000000000007',
    'Task B7',
    'Completed, Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
    '2025-01-02',
    NULL
),
(
    '00000000-0000-40aa-9002-000000000008',
    'Task B8',
    'Completed, Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
    '2025-01-02',
    '2025-01-03'
);


/* PROJECTS */
INSERT INTO data.projects -- Project 0
(
    project_id,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9000-000000000000',
	'No Title',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.projects -- Project A
(
    project_id,
	project_title,
    notes,

    start_date,
    start_time,
    deadline,

    user_id
)
VALUES
(
    '00000000-0000-40bb-9001-000000000001',
	'Project A1',
	'No start date, No start time, No deadline',

	NULL,
	NULL,
	NULL,

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9001-000000000002',
	'Project A2',
	'Start date, No start time, No deadline',

	'2026-01-01',
	NULL,
	NULL,

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9001-000000000003',
	'Project A3',
	'Start date, Start time, No deadline',

	'2026-01-01',
	'12:00:00',
	NULL,

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9001-000000000004',
	'Project A4',
	'No start date, No start time, Deadline',

	NULL,
	NULL,
	'2026-02-01',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9001-000000000005',
	'Project A5',
	'Start date, No start time, Deadline',

	'2026-01-01',
	NULL,
	'2026-02-01',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9001-000000000006',
	'Project A6',
	'Start date, Start time, Deadline',

	'2026-01-01',
	'12:00:00',
	'2026-02-01',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.projects -- Project B
(
    project_id,
	project_title,
	notes,

	user_id,
	completed_on,
	logged_on,
	trashed_on
)
VALUES
(
    '00000000-0000-40bb-9002-000000000001',
    'Project B1',
    'No Completed, No Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
	NULL,
	NULL
),
(
    '00000000-0000-40bb-9002-000000000002',
    'Project B2',
    'No Completed, No Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    NULL,
    '2025-01-03'
),
(
    '00000000-0000-40bb-9002-000000000003',
    'Project B3',
    'No Completed, Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    '2025-01-02',
    NULL
),
(
    '00000000-0000-40bb-9002-000000000004',
    'Project B4',
    'No Completed, Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    NULL,
    '2025-01-02',
    '2025-01-03'
),
(
    '00000000-0000-40bb-9002-000000000005',
    'Project B5',
    'Completed, No Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
	NULL,
	NULL
),
(
    '00000000-0000-40bb-9002-000000000006',
    'Project B6',
    'Completed, No Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
	NULL,
    '2025-01-03'
),
(
    '00000000-0000-40bb-9002-000000000007',
    'Project B7',
    'Completed, Logged, No Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
    '2025-01-02',
    NULL
),
(
    '00000000-0000-40bb-9002-000000000008',
    'Project B8',
    'Completed, Logged, Trashed',

    (SELECT user_id FROM auth.users WHERE email='testuser1@email.com'),
    '2025-01-01',
    '2025-01-02',
    '2025-01-03'
);

INSERT INTO data.projects -- Project C
(
    project_id,
	project_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9003-000000000000',
	'Project C',
	'No Area',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.tasks -- Task C
(
    task_id,
	task_title,
	notes,

	project_id,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9003-000000000001',
	'Task C1',
	'In Project C',

	(SELECT project_id FROM data.projects WHERE project_title='Project C'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9003-000000000002',
	'Task C2',
	'In Project C',

	(SELECT project_id FROM data.projects WHERE project_title='Project C'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9003-000000000003',
	'Task C3',
	'In Project C',

	(SELECT project_id FROM data.projects WHERE project_title='Project C'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9003-000000000004',
	'Task C4',
	'In Project C',

	(SELECT project_id FROM data.projects WHERE project_title='Project C'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9003-000000000005',
	'Task C5',
	'In Project C',

	(SELECT project_id FROM data.projects WHERE project_title='Project C'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);


/* AREAS */
INSERT INTO data.areas -- Area 0
(
    area_id,
	user_id
)
VALUES
(
    '00000000-0000-40cc-9000-000000000000',
	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.areas -- Area D
(
    area_id,
	area_name,

	user_id
)
VALUES
(
    '00000000-0000-40cc-9004-000000000000',
	'Area D',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.projects -- Project D
(
    project_id,
	project_title,
	notes,

	area_id,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9004-000000000000',
	'Project D',
	'In Area D',

	(SELECT area_id FROM data.areas WHERE area_name='Area D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.tasks -- Task D
(
    task_id,
	task_title,
	notes,

	project_id,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9004-000000000001',
	'Task D1',
	'In Project D',

	(SELECT project_id FROM data.projects WHERE project_title='Project D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9004-000000000002',
	'Task D2',
	'In Project D',

	(SELECT project_id FROM data.projects WHERE project_title='Project D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9004-000000000003',
	'Task D3',
	'In Project D',

	(SELECT project_id FROM data.projects WHERE project_title='Project D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9004-000000000004',
	'Task D4',
	'In Project D',

	(SELECT project_id FROM data.projects WHERE project_title='Project D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9004-000000000005',
	'Task D5',
	'In Project D',

	(SELECT project_id FROM data.projects WHERE project_title='Project D'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);

INSERT INTO data.areas -- Area E
(
    area_id,
	area_name,

	user_id
)
VALUES
(
    '00000000-0000-40cc-9005-000000000000',
	'Area E',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.tasks -- Task E
(
    task_id,
	task_title,
	notes,

	area_id,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9005-000000000001',
	'Task E1',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9005-000000000002',
	'Task E2',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9005-000000000003',
	'Task E3',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9005-000000000004',
	'Task E4',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9005-000000000005',
	'Task E5',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.projects -- Project E
(
    project_id,
	project_title,
	notes,

	area_id,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9005-000000000001',
	'Project E1',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9005-000000000002',
	'Project E2',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9005-000000000003',
	'Project E3',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9005-000000000004',
	'Project E4',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9005-000000000005',
	'Project E5',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.projects -- Project F
(
    project_id,
	project_title,
	notes,

	area_id,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9006-000000000000',
	'Project F',
	'In Area E',

	(SELECT area_id FROM data.areas WHERE area_name='Area E'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.tasks -- Task F
(
    task_id,
	task_title,
	notes,

	project_id,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9006-000000000001',
	'Task F1',
	'In Project F',

	(SELECT project_id FROM data.projects WHERE project_title='Project F'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9006-000000000002',
	'Task F2',
	'In Project F',

	(SELECT project_id FROM data.projects WHERE project_title='Project F'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9006-000000000003',
	'Task F3',
	'In Project F',

	(SELECT project_id FROM data.projects WHERE project_title='Project F'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9006-000000000004',
	'Task F4',
	'In Project F',

	(SELECT project_id FROM data.projects WHERE project_title='Project F'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9006-000000000005',
	'Task F5',
	'In Project F',

	(SELECT project_id FROM data.projects WHERE project_title='Project F'),

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);


/* TAGS */
INSERT INTO data.tags
(
    tag_id,
	tag_label,
	category,

	color,

	user_id
)
VALUES
(
	'00000000-0000-40dd-9000-000000000000',
	'Sample Tag',
	NULL,

	'#000000',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
-- Tag 0
(
    '00000000-0000-40dd-9000-000000000001',
	'Important',
	NULL,

	'#ffe600',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9000-000000000002',
	'Urgent',
	NULL,

	'#ff0000',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
-- Tag A
(
    '00000000-0000-40dd-9001-000000000001',
	'Low',
	'Priority',

	'#0aab20',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9001-000000000002',
	'Mid',
	'Priority',

	'#faea05',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9001-000000000003',
	'High',
	'Priority',

	'#fa1d05',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
-- Tag B
(
    '00000000-0000-40dd-9002-000000000001',
	'Backlog',
	'Scrum',

	'#757171',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9002-000000000002',
	'To-do',
	'Scrum',

	'#e3c905',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9002-000000000003',
	'In-progress',
	'Scrum',

	'#f59b0a',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9002-000000000004',
	'Done',
	'Scrum',

	'#02d102',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
-- Tag C
(
    '00000000-0000-40dd-9003-000000000001',
	'Class 1',
	'School',

	'#d11702',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9003-000000000002',
	'Class 2',
	'School',

	'#e08002',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9003-000000000003',
	'Class 3',
	'School',

	'#fcf000',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9003-000000000004',
	'Class 4',
	'School',

	'#02cc02',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40dd-9003-000000000005',
	'Class 5',
	'School',

	'#0b2fe3',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);


/* TASK WITH TAGS */
INSERT INTO data.tasks -- Task G
(
    task_id,
	task_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9007-000000000001',
	'Task G1',
	'Priority: Low',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9007-000000000002',
	'Task G2',
	'Priority: Mid',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9007-000000000003',
	'Task G3',
	'Priority: High',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.task_tags
(
	task_id,
	tag_id
)
VALUES
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task G1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Low' AND category='Priority')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task G2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Mid' AND category='Priority')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task G3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='High' AND category='Priority')
);

INSERT INTO data.tasks -- Task H
(
    task_id,
	task_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9008-000000000001',
	'Task H1',
	'School: Class 1',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9008-000000000002',
	'Task H2',
	'School: Class 2',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9008-000000000003',
	'Task H3',
	'School: Class 3',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9008-000000000004',
	'Task H4',
	'School: Class 4',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9008-000000000005',
	'Task H5',
	'School: Class 5',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.task_tags
(
	task_id,
	tag_id
)
VALUES
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task H1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 1' AND category='School')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task H2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 2' AND category='School')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task H3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 3' AND category='School')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task H4'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 4' AND category='School')
),
(
	(SELECT	task_id FROM data.tasks WHERE task_title='Task H5'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 5' AND category='School')
);

INSERT INTO data.tasks -- Task I
(
    task_id,
	task_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40aa-9009-000000000001',
	'Task I1',
	'Scrum: Backlog; Important',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9009-000000000002',
	'Task I2',
	'Scrum: To-do; Urgent',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9009-000000000003',
	'Task I3',
	'Scrum: In-progress',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40aa-9009-000000000004',
	'Task I4',
	'Scrum: Done',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.task_tags
(
	task_id,
	tag_id
)
VALUES
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Backlog' AND category='Scrum')
),
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Important' AND category IS NULL)
),
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='To-do' AND category='Scrum')
),
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Urgent' AND category IS NULL)
),
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='In-progress' AND category='Scrum')
),
(
	(SELECT task_id FROM data.tasks WHERE task_title='Task I4'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Done' AND category='Scrum')
);


/* PROJECT WITH TAGS */
INSERT INTO data.projects -- Project G
(
    project_id,
	project_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9007-000000000001',
	'Project G1',
	'Priority: Low',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9007-000000000002',
	'Project G2',
	'Priority: Mid',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9007-000000000003',
	'Project G3',
	'Priority: High',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.project_tags
(
	project_id,
	tag_id
)
VALUES
(
	(SELECT project_id FROM data.projects WHERE project_title='Project G1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Low' AND category='Priority')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project G2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Important' AND category IS NULL)
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project G2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Mid' AND category='Priority')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project G3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Urgent' AND category IS NULL)
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project G3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='High' AND category='Priority')
);

INSERT INTO data.projects -- Project H
(
    project_id,
	project_title,
	notes,

	user_id
)
VALUES
(
    '00000000-0000-40bb-9008-000000000001',
	'Project H1',
	'School: Class 1',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9008-000000000002',
	'Project H2',
	'School: Class 2',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9008-000000000003',
	'Project H3',
	'School: Class 3',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9008-000000000004',
	'Project H4',
	'School: Class 4',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
),
(
    '00000000-0000-40bb-9008-000000000005',
	'Project H5',
	'School: Class 5',

	(SELECT user_id FROM auth.users WHERE email='testuser1@email.com')
);
INSERT INTO data.project_tags
(
	project_id,
	tag_id
)
VALUES
(
	(SELECT project_id FROM data.projects WHERE project_title='Project H1'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 1' AND category='School')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project H2'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 2' AND category='School')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project H3'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 3' AND category='School')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project H4'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 4' AND category='School')
),
(
	(SELECT project_id FROM data.projects WHERE project_title='Project H5'),
	(SELECT tag_id FROM data.tags WHERE tag_label='Class 5' AND category='School')
);
