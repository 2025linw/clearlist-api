/*
This SQL script is an up migration script for Todo List Database
*/


/*
Adds authentication schema and adjust permissions::
    Give API user (todo_app) access to use schema and grant data manipulation functions to tables in schema by default
*/
CREATE SCHEMA IF NOT EXISTS auth;
GRANT USAGE ON SCHEMA auth TO todo_app;
ALTER DEFAULT PRIVILEGES
IN SCHEMA auth
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO todo_app;

/*
Adds data schema and adjust permissions:
    Give API user (todo_app) access to use schema and grant data manipulation functions to tables in schema by default
*/
CREATE SCHEMA IF NOT EXISTS data;
GRANT USAGE ON SCHEMA data TO todo_app;
ALTER DEFAULT PRIVILEGES
IN SCHEMA data
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO todo_app;

/*
Adds functions to verify that a task/project and tag being related are created by the same user
*/
CREATE OR REPLACE FUNCTION check_task_tag_user_consistency()
RETURNS trigger AS $$
DECLARE
    task_user UUID;
    tag_user UUID;
BEGIN
    SELECT user_id INTO task_user FROM data.tasks WHERE task_id = NEW.task_id;
    SELECT user_id INTO tag_user FROM data.tags WHERE tag_id = NEW.tag_id;

    IF task_user IS NULL OR tag_user IS NULL THEN
        RAISE EXCEPTION 'Task or Tag not found';
    END IF;

    IF task_user != tag_user THEN
        RAISE EXCEPTION 'Task and Tag must belong to the same user';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION check_project_tag_user_consistency()
RETURNS trigger AS $$
DECLARE
    project_user UUID;
    tag_user UUID;
BEGIN
    SELECT user_id INTO project_user FROM data.projects WHERE project_id = NEW.project_id;
    SELECT user_id INTO tag_user FROM data.tags WHERE tag_id = NEW.tag_id;

    IF project_user IS NULL OR tag_user IS NULL THEN
        RAISE EXCEPTION 'Project or Tag not found';
    END IF;

    IF project_user != tag_user THEN
        RAISE EXCEPTION 'Project and Tag must belong to the same user';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


/*
Create users tables in auth schema
*/
CREATE TABLE IF NOT EXISTS auth.users
(
	user_id uuid DEFAULT gen_random_uuid(),

	email varchar(320) NOT NULL UNIQUE,
	password_hash text NOT NULL,

    -- TODO: add user first and last name
    first_name text,
    last_name text,

	created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_on timestamptz(0),

	PRIMARY KEY (user_id)
);

/*
Create areas table in data schema

Area must be associated with an existing user
*/
CREATE TABLE IF NOT EXISTS data.areas
(
	area_id	uuid DEFAULT gen_random_uuid(),

	area_name text,
	icon_url text,

	user_id uuid NOT NULL,

    created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_on timestamptz(0),

	PRIMARY	KEY (area_id),
	FOREIGN	KEY (user_id) REFERENCES auth.users(user_id)
);

/*
Create projects table in data schema

Project must be associated with an existing user
*/
CREATE TABLE IF NOT EXISTS data.projects
(
	project_id uuid DEFAULT gen_random_uuid(),

    project_title text,
    notes text,
    start_date date,
	start_time time(0),
	deadline date,
	completed_on timestamptz(0),
	logged_on timestamptz(0),

    area_id uuid,

    user_id	uuid NOT NULL,

	created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_on timestamptz(0),

	PRIMARY KEY (project_id),
    FOREIGN KEY (area_id) REFERENCES data.areas(area_id),
	FOREIGN	KEY (user_id) REFERENCES auth.users(user_id)
);

/*
Create tasks table in data schema

Task must be associated with an existing user
*/
CREATE TABLE IF NOT EXISTS data.tasks
(
	task_id uuid DEFAULT gen_random_uuid(),

    task_title text,
    notes text,
    start_date date,
	start_time time(0),
	deadline date,
	completed_on timestamptz(0),
	logged_on timestamptz(0),

    area_id uuid,
	project_id uuid,

    user_id	uuid NOT NULL,

	created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_on timestamptz(0),

	PRIMARY KEY (task_id),
	FOREIGN KEY (area_id) REFERENCES data.areas(area_id) ON DELETE SET NULL,
	FOREIGN KEY (project_id) REFERENCES data.projects(project_id) ON DELETE SET NULL,
	FOREIGN	KEY (user_id) REFERENCES auth.users(user_id)
);

/*
Create tags table in data schema
*/
CREATE TABLE IF NOT EXISTS data.tags
(
	tag_id uuid DEFAULT gen_random_uuid(),

	tag_label text NOT NULL,
	color varchar(7) CHECK (color IS NULL OR color ~* '^#[a-f0-9]{6}$'),

	category varchar(255),

	user_id uuid NOT NULL,
    created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY KEY (tag_id),
	FOREIGN KEY (user_id) REFERENCES auth.users(user_id)
);

/*
Create project-tags relation table in data schema

If associated project or tag is deleted, delete the relationship
*/
CREATE TABLE IF NOT EXISTS data.project_tags
(
	project_id uuid,
	tag_id uuid,

	PRIMARY KEY (project_id, tag_id),
	FOREIGN KEY (project_id) REFERENCES data.projects(project_id) ON DELETE CASCADE,
	FOREIGN KEY (tag_id) REFERENCES data.tags(tag_id) ON DELETE CASCADE
);

CREATE OR REPLACE TRIGGER check_user_id_match_trigger
BEFORE INSERT OR UPDATE ON data.project_tags
FOR EACH ROW EXECUTE FUNCTION check_project_tag_user_consistency();

/*
Create task-tags relation table in data schema

If associated task or tag is deleted, delete the relationship
*/
CREATE TABLE IF NOT EXISTS data.task_tags
(
	task_id uuid,
	tag_id uuid,

	PRIMARY KEY (task_id, tag_id),
	FOREIGN KEY (task_id) REFERENCES data.tasks(task_id) ON DELETE CASCADE,
	FOREIGN KEY (tag_id) REFERENCES data.tags(tag_id) ON DELETE CASCADE
);

CREATE OR REPLACE TRIGGER check_user_id_match_trigger
BEFORE INSERT OR UPDATE ON data.task_tags
FOR EACH ROW EXECUTE FUNCTION check_task_tag_user_consistency();
