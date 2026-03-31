CREATE TABLE app.tasks (
    id UUID PRIMARY KEY DEFAULT app.gen_random_uuid(),

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

-- Create index for Task owner ids
CREATE INDEX idx_tasks_owner
ON app.tasks (created_by)
WHERE deleted_at IS NULL;

-- Create indexes for deleted Tasks
CREATE INDEX ON app.tasks (id) WHERE deleted_at IS NOT NULL;
