-- Allow API database user to 'app' schema and data manipulation to required tables
GRANT USAGE ON SCHEMA app TO cl_api;

GRANT SELECT, INSERT, UPDATE, DELETE ON
app.tasks,
app.tags,
app.task_tags
TO cl_api;

-- Allow API database user to 'auth' schema and SELECT to session and user table
GRANT USAGE ON SCHEMA auth TO cl_api;

GRANT SELECT ON
auth.session,
auth.user
TO cl_api;
