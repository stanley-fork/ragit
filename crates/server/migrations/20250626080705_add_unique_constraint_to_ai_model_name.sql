-- This is a design mistake. At first, I wanted to allow users to use different models
-- with the same name. Users use their models to chat. They only care about their models,
-- and others' models might have the same name. The `id` field was to deduplicate exact
-- same models.
-- But now, we're not working on the chat feature anymore. Instead, it's a model store
-- where ragit users fetch new models. So there must be no name conflict.
DROP INDEX IF EXISTS ai_model_by_name;
CREATE UNIQUE INDEX IF NOT EXISTS ai_model_by_name ON ai_model ( name );
