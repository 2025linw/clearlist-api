-- Allow API user to 'app' schema and data manipulation to required tables
GRANT USAGE ON SCHEMA app TO cl_rw;

GRANT SELECT, INSERT, UPDATE, DELETE ON
app.tasks,
app.tags,
app.task_tags
TO cl_rw;

-- Allow API user to 'auth' schema with only read permissions
GRANT USAGE ON SCHEMA auth TO cl_rw;

GRANT SELECT ON
auth.session,
auth.user
TO cl_rw;
