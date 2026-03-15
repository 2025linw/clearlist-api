CREATE SCHEMA IF NOT EXISTS app;

/* Schema */

-- Tag Table
CREATE TABLE app.tags (
    id UUID PRIMARY KEY DEFAULT uuidv4(),

    label VARCHAR (255) NOT NULL,
    category VARCHAR (255)
);

-- Task Table
CREATE TABLE app.tasks (
    id UUID PRIMARY KEY DEFAULT uuidv4(),

    title VARCHAR (255) NOT NULL,
    notes TEXT,
    start_date DATE,
    start_time TIME (0),
    deadline DATE
);

-- Task-Tag Table
CREATE TABLE app.task_tags (
    task_id UUID NOT NULL,
    tag_id UUID NOT NULL,

    PRIMARY KEY (task_id, tag_id),
    FOREIGN KEY (task_id) REFERENCES app.tasks (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES app.tags (id) ON DELETE CASCADE
);


/* User Permission */

GRANT USAGE ON SCHEMA app TO cl_api;
GRANT USAGE ON SCHEMA auth TO cl_api;

GRANT SELECT, INSERT, UPDATE, DELETE ON
app.tasks,
app.tags,
app.task_tags
TO cl_api;

GRANT SELECT ON
auth.session,
auth.user
TO cl_api;
