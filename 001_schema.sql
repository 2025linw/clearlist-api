CREATE SCHEMA IF NOT EXISTS app;

-- Tag Table
CREATE TABLE app.tags (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),

    label varchar(255) NOT NULL,
    category varchar(255),

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,

    created_by uuid NOT NULL,

    FOREIGN KEY (created_by) REFERENCES auth.user (id)
);

-- Task Table
CREATE TABLE app.tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    title varchar(255) NOT NULL,
    notes text,
    -- TODO: create trigger to limit only start_date OR start_at
    start_on date,
    start_at timestamp with time zone,
    deadline date,

    deleted_at timestamp with time zone,

    created_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL default CURRENT_TIMESTAMP,

    created_by uuid NOT NULL,

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

-- Create index for Task owner ids
CREATE INDEX idx_tasks_owner
ON app.tasks (created_by)
WHERE deleted_at IS NULL;

-- Create indexes for deleted Tasks and Tags
CREATE INDEX ON app.tasks (id) WHERE deleted_at IS NOT NULL;
CREATE INDEX ON app.tags (id) WHERE deleted_at IS NOT NULL;

-- Trigger to ensure that no task or tag is 'deleted' when adding task-tags
CREATE OR REPLACE FUNCTION check_task_tag_not_deleted()
RETURNS TRIGGER AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM app.tasks t, app.tags g
        WHERE t.id = NEW.task_id
        AND g.id = NEW.tag_id
        AND (t.deleted_at IS NOT NULL OR g.deleted_at IS NOT NULL)
    ) THEN
        RAISE EXCEPTION 'Cannot link deleted task or tag';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trig_check_task_tag_not_deleted
BEFORE INSERT ON app.task_tags
FOR EACH ROW
EXECUTE FUNCTION check_task_tag_not_deleted();
