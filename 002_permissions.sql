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
