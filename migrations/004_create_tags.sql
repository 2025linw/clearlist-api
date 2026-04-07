CREATE TABLE app.tags (
    id uuid PRIMARY KEY DEFAULT app.gen_random_uuid(),

    label varchar(255) NOT NULL,
    category varchar(255),

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
    FOREIGN KEY (task_id) REFERENCES app.tasks (id),
    FOREIGN KEY (tag_id) REFERENCES app.tags (id) ON DELETE CASCADE
);

-- Trigger to ensure that no task or tag is 'deleted' when adding task-tags
CREATE OR REPLACE FUNCTION app.check_task_not_deleted()
RETURNS TRIGGER AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM app.tasks t
        WHERE t.id = NEW.task_id
        AND t.deleted_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Cannot link with deleted task';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trig_check_task_not_deleted
BEFORE INSERT ON app.task_tags
FOR EACH ROW
EXECUTE FUNCTION app.check_task_not_deleted();
