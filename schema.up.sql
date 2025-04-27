/*
This SQL script is an up migration script for Todo List Database
*/

/*
Adds case insesitive text to database

By default, most TEXT in tables are all citext.
However, this may change to allow for granular control over
searching
*/
CREATE EXTENSION citext;


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
Create users tables in auth schema
*/
CREATE TABLE IF NOT EXISTS auth.users
(
	user_id uuid DEFAULT gen_random_uuid(),

	email varchar(320) NOT NULL UNIQUE,
	password_hash text NOT NULL,

    -- TODO: add user first and last name

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

	area_name citext,
	icon_url text,

	user_id uuid NOT NULL,
    created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY	KEY(area_id),
	FOREIGN	KEY(user_id) REFERENCES auth.users(user_id)
);

/*
Create projects table in data schema

Project must be associated with an existing user
*/
CREATE TABLE IF NOT EXISTS data.projects
(
	project_id uuid DEFAULT gen_random_uuid(),

    project_title citext,
    notes text,
    start_date date,
	start_time time(0),
	deadline date,
	completed_on timestamptz(0),
	logged_on timestamptz(0),
	trashed_on timestamptz(0),

    area_id uuid,

    user_id	uuid NOT NULL,
	created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY KEY(project_id),
    FOREIGN KEY(area_id) REFERENCES data.areas(area_id),
	FOREIGN	KEY(user_id) REFERENCES auth.users(user_id)
);

/*
Create tasks table in data schema

Task must be associated with an existing user
*/
CREATE TABLE IF NOT EXISTS data.tasks
(
	task_id uuid DEFAULT gen_random_uuid(),

    task_title citext,
    notes text,
    start_date date,
	start_time time(0),
	deadline date,
	completed_on timestamptz(0),
	logged_on timestamptz(0),
	trashed_on timestamptz(0),

    area_id uuid,
	project_id uuid,

    user_id	uuid NOT NULL,
	created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY KEY(task_id),
	FOREIGN KEY(area_id) REFERENCES data.areas(area_id),
	FOREIGN KEY(project_id) REFERENCES data.projects(project_id),
	FOREIGN	KEY(user_id) REFERENCES auth.users(user_id)
);

/*
Create tags table in data schema
*/
CREATE TABLE IF NOT EXISTS data.tags
(
	tag_id uuid DEFAULT gen_random_uuid(),

	tag_label citext NOT NULL,
	color varchar(7) CHECK (color IS NULL OR color ~* '^#[a-f0-9]{6}$'),

	category varchar(255),

	user_id uuid NOT NULL,
    created_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_on timestamptz(0) DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY KEY(tag_id),
	FOREIGN KEY(user_id) REFERENCES auth.users(user_id)
);

/*
Create project-tags relation table in data schema

If associated project or tag is deleted, delete the relationship
*/
CREATE TABLE IF NOT EXISTS data.project_tags
(
	project_id uuid,
	tag_id uuid,

	PRIMARY KEY(project_id, tag_id),
	FOREIGN KEY(project_id) REFERENCES data.projects(project_id) ON DELETE CASCADE,
	FOREIGN KEY(tag_id) REFERENCES data.tags(tag_id) ON DELETE CASCADE
);

/*
Create task-tags relation table in data schema

If associated task or tag is deleted, delete the relationship
*/
CREATE TABLE IF NOT EXISTS data.task_tags
(
	task_id uuid,
	tag_id uuid,

	PRIMARY KEY(task_id, tag_id),
	FOREIGN KEY(task_id) REFERENCES data.tasks(task_id) ON DELETE CASCADE,
	FOREIGN KEY(tag_id) REFERENCES data.tags(tag_id) ON DELETE CASCADE
);
