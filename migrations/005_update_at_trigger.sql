-- Trigger to auto update updated_at fields for resources
CREATE OR REPLACE FUNCTION app.set_updated_at()
RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD
        AND NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at = CURRENT_TIMESTAMP;
    END IF;

    RETURN NEW;
END;
$$ language plpgsql;

CREATE TRIGGER trig_set_task_updated_at
BEFORE UPDATE ON app.tasks
FOR EACH ROW
EXECUTE FUNCTION app.set_updated_at();

CREATE TRIGGER trig_set_tag_updated_at
BEFORE UPDATE ON app.tags
FOR EACH ROW
EXECUTE FUNCTION app.set_updated_at();
