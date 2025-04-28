/*
This SQL script is a down migration script for Todo List Database
*/

/*
Drop auth and data schemas, and all tables associated with them

TODO: Should add more granular control over the removal
*/
DROP SCHEMA IF EXISTS auth, data CASCADE;
