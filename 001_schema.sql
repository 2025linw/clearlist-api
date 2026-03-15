CREATE SCHEMA IF NOT EXISTS app;

-- Tag Table
CREATE TABLE app.tags (
    id uuid PRIMARY KEY DEFAULT uuidv4(),

    label varchar(255) NOT NULL,
    category varchar(255),

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL
);

-- Task Table
CREATE TABLE app.tasks (
    id UUID PRIMARY KEY DEFAULT uuidv4(),

    title varchar(255) NOT NULL,
    notes text,
    start_date date,
    start_time time with time zone,
    deadline date,

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL
);

-- Task-Tag Table
CREATE TABLE app.task_tags (
    task_id uuid NOT NULL,
    tag_id uuid NOT NULL,

    PRIMARY KEY (task_id, tag_id),
    FOREIGN KEY (task_id) REFERENCES app.tasks (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES app.tags (id) ON DELETE CASCADE
);
