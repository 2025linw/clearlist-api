CREATE SCHEMA IF NOT EXISTS clear_list;

/* Schema */

-- Tag Table
CREATE TABLE clear_list.tags (
    id UUID DEFAULT gen_random_uuid(),

    label VARCHAR (255) NOT NULL,
    category VARCHAR (255),

    PRIMARY KEY (id)
);

-- Task Table
CREATE TABLE clear_list.tasks (
    id UUID DEFAULT gen_random_uuid(),

    title VARCHAR (255) NOT NULL,
    notes TEXT,
    start_date DATE,
    start_time TIME (0),
    deadline DATE,

    PRIMARY KEY (id)
);

-- Task-Tag Table
CREATE TABLE clear_list.task_tags (
    task_id UUID,
    tag_id UUID,

    PRIMARY KEY (task_id, tag_id),
    FOREIGN KEY (task_id) REFERENCES clear_list.tasks (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES clear_list.tags (id) ON DELETE CASCADE
);


/* User Permission */

GRANT USAGE ON SCHEMA clear_list TO cl_api;

GRANT SELECT, INSERT, UPDATE, DELETE
ON ALL TABLES IN SCHEMA clear_list
TO cl_api;

ALTER DEFAULT PRIVILEGES
IN SCHEMA clear_list
GRANT SELECT, INSERT, UPDATE, DELETE
ON TABLES TO cl_api;
