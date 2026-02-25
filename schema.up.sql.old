/*
This SQL script is an up migration script for Todo List Database
*/


/*
Adds clear_list schema
*/
CREATE SCHEMA IF NOT EXISTS clear_list AUTHORIZATION todo_app;


/*
Function to verify that a task-tag or project-tag relation is associated with the same user
*/
CREATE OR REPLACE FUNCTION check_task_tag_user_consistency()
RETURNS trigger AS $$
DECLARE
    task_user UUID;
    tag_user UUID;
BEGIN
    SELECT user_id INTO task_user FROM clear_list.tasks WHERE task_id = NEW.task_id;
    SELECT user_id INTO tag_user FROM clear_list.tags WHERE tag_id = NEW.tag_id;

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
    SELECT user_id INTO project_user FROM clear_list.projects WHERE project_id = NEW.project_id;
    SELECT user_id INTO tag_user FROM clear_list.tags WHERE tag_id = NEW.tag_id;

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
Create users tables in clear_list schema
*/
CREATE TABLE IF NOT EXISTS clear_list.user
(
	user_id uuid DEFAULT gen_random_uuid(),

	email varchar(320) NOT NULL UNIQUE,
    given_name varchar(255),
    family_name varchar(255),

	created_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at timestamp(0) with time zone,

	PRIMARY KEY (user_id)
);

/*
Create tags table in clear_list schema
*/
CREATE TABLE IF NOT EXISTS clear_list.tag
(
	tag_id uuid UNIQUE NOT NULL DEFAULT gen_random_uuid(),

	title text NOT NULL,
	color varchar(7) CHECK (color IS NULL OR color ~* '^#[a-f0-9]{6}$'),

	category varchar(255),

	user_id uuid NOT NULL,
    created_at timestamp(0) with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp(0) with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,

	PRIMARY KEY (tag_id),
	FOREIGN KEY (user_id) REFERENCES clear_list.users(user_id)
);

/*
Create areas table in clear_list schema
*/
CREATE TABLE IF NOT EXISTS clear_list.area
(
	area_id	uuid UNIQUE NOT NULL DEFAULT gen_random_uuid(),

	title varchar(255),
	icon_path text,

	user_id uuid NOT NULL,

    created_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at timestamp(0) with time zone,

	PRIMARY	KEY (area_id),
	FOREIGN	KEY (user_id) REFERENCES clear_list.users(user_id)
);

/*
Create projects table in clear_list schema
*/
CREATE TABLE IF NOT EXISTS clear_list.project
(
	project_id uuid UNIQUE NOT NULL DEFAULT gen_random_uuid(),

    title varchar(255),
    notes text,

    start_date date,
	start_time time(0),
	deadline date,

	completed_at timestamp(0) with time zone,
	logged_at timestamp(0) with time zone,

    area_id uuid,

    user_id	uuid NOT NULL,

	created_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at timestamp(0) with time zone,

	PRIMARY KEY (project_id),
    FOREIGN KEY (area_id) REFERENCES clear_list.areas(area_id) ON DELETE SET NULL,
	FOREIGN	KEY (user_id) REFERENCES clear_list.users(user_id)
);

/*
Create tasks table in clear_list schema
*/
CREATE TABLE IF NOT EXISTS clear_list.task
(
	task_id uuid UNIQUE NOT NULL DEFAULT gen_random_uuid(),

    title varchar(255),
    notes text,

    start_date date,
	start_time time(0),
	deadline date,

	completed_at timestamp(0) with time zone,
	logged_at timestamp(0) with time zone,

    area_id uuid,
	project_id uuid,

    user_id	uuid NOT NULL,

	created_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(0) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at timestamp(0) with time zone,

	PRIMARY KEY (task_id),
	FOREIGN KEY (area_id) REFERENCES clear_list.areas(area_id) ON DELETE SET NULL,
	FOREIGN KEY (project_id) REFERENCES clear_list.projects(project_id) ON DELETE SET NULL,
	FOREIGN	KEY (user_id) REFERENCES clear_list.users(user_id)
);

/*
Create project-tags relation table in clear_list schema

Ensure that the project and tag are each associated with the same person
*/
CREATE TABLE IF NOT EXISTS clear_list.project_tagging
(
	project_id uuid,
	tag_id uuid,

	PRIMARY KEY (project_id, tag_id),
	FOREIGN KEY (project_id) REFERENCES clear_list.projects(project_id) ON DELETE CASCADE,
	FOREIGN KEY (tag_id) REFERENCES clear_list.tags(tag_id) ON DELETE CASCADE
);
CREATE OR REPLACE TRIGGER check_user_id_match_trigger
BEFORE INSERT OR UPDATE ON clear_list.project_tags
FOR EACH ROW EXECUTE FUNCTION check_project_tag_user_consistency();

/*
Create task-tags relation table in clear_list schema

Ensure that the task and tag are each associated with the same person
*/
CREATE TABLE IF NOT EXISTS clear_list.task_tagging
(
	task_id uuid,
	tag_id uuid,

	PRIMARY KEY (task_id, tag_id),
	FOREIGN KEY (task_id) REFERENCES clear_list.tasks(task_id) ON DELETE CASCADE,
	FOREIGN KEY (tag_id) REFERENCES clear_list.tags(tag_id) ON DELETE CASCADE
);
CREATE OR REPLACE TRIGGER check_user_id_match_trigger
BEFORE INSERT OR UPDATE ON clear_list.task_tags
FOR EACH ROW EXECUTE FUNCTION check_task_tag_user_consistency();
