CREATE SCHEMA IF NOT EXISTS app;

-- Tag Table
CREATE TABLE app.tags (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),

    label varchar(255) NOT NULL,
    category varchar(255),

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,

    created_by text NOT NULL,

    FOREIGN KEY (created_by) REFERENCES auth.user (id)
);

-- Task Table
CREATE TABLE app.tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title varchar(255) NOT NULL,
    notes text,
    -- TODO: create trigger to limit only start_date OR start_at
    start_date date,
    start_at timestamp with time zone,
    deadline date,

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,

    created_by text NOT NULL,

    FOREIGN KEY (created_by) REFERENCES auth.user (id)
);

-- Task-Tag Table
CREATE TABLE app.task_tags (
    task_id uuid NOT NULL,
    tag_id uuid NOT NULL,

    PRIMARY KEY (task_id, tag_id),
    FOREIGN KEY (task_id) REFERENCES app.tasks (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES app.tags (id) ON DELETE CASCADE
);

-- Task Index by Owner
CREATE INDEX idx_tasks_owner
ON app.tasks (created_by)
WHERE deleted_at IS NULL;
