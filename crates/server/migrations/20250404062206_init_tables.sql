-- NOTE: postgresql doesn't allow me to use `user` as a name of a table
CREATE TABLE IF NOT EXISTS user_ (
    id TEXT PRIMARY KEY,

    name TEXT,
    email TEXT NOT NULL,
    readme TEXT,
    public BOOLEAN NOT NULL,
    is_admin BOOLEAN NOT NULL,

    salt TEXT NOT NULL,
    password TEXT NOT NULL,

    -- For now, it only uses sha3_256. This field is for forward-compatibility.
    password_hash_type TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL,
    last_login_at TIMESTAMPTZ
);

-- At first, I wanted to implement chat in ragit-server. Users register their api key to the
-- server and use the models. That's why there are 2 tables for ai models: `user_ai_model` and
-- `ai_model`. `ai_model` has information about llama, gpt and claude. `user_ai_model` has
-- user A's api key for llama, user B's api key for llama, user B's api key for gpt, ...
--
-- For now, we're not adding chat interface to ragithub, so we're only using `ai_model`. But
-- I'll add chat interface someday, so I'll keep `user_ai_model` table.

-- AI model for chat.
-- It's user's reponsibility to register a valid model and api_key.
CREATE TABLE IF NOT EXISTS user_ai_model (
    id SERIAL PRIMARY KEY,
    user_ TEXT NOT NULL,

    -- TODO: we have to make sure that the models of a user
    --       must have a unique name. How do we do that?
    ai_model_id TEXT NOT NULL,

    -- TODO: is it okay to store api_key in plaintext?
    --       how about keeping another table (encrypted) on disk?
    --       how about using `PGP_SYM_ENCRYPT` and `PGP_SYM_DECRYPT`?
    api_key TEXT,
    default_model BOOLEAN NOT NULL,
    added_at TIMESTAMPTZ NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS user_ai_model_by_user ON user_ai_model ( user_, ai_model_id );

CREATE TABLE IF NOT EXISTS ai_model (
    id TEXT PRIMARY KEY,  -- hash value of model name and metadata

    -- `id` field is a hash value of these 4 fields: `name`, `api_name`, `api_provider`, `api_url`.

    -- `name` is  for humans, and `api_name` is for apis.
    -- For example, llama3.3's name for groq's api is "llama-3.3-70b-versatile" and
    -- for human it's "llama3.3-70b-groq".
    name TEXT NOT NULL,
    api_name TEXT NOT NULL,

    -- openai | anthropic | cohere | google
    api_provider TEXT NOT NULL,

    -- It's for openai-compatible apis.
    api_url TEXT,

    can_read_images BOOLEAN NOT NULL,

    -- dollars per million tokens
    -- if it's a free model, it's 0
    input_price DOUBLE PRECISION NOT NULL,
    output_price DOUBLE PRECISION NOT NULL,

    explanation TEXT,

    -- When users download this model, ragit will use this env var to
    -- find the api key.
    api_env_var TEXT,

    -- for easier search
    tags TEXT[] NOT NULL,

    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS ai_model_by_name ON ai_model ( name );

-- `name`, `description` and `website` are managed by ragit-server, but `readme` is managed by `rag meta`.
-- That's how git and github works!
CREATE TABLE IF NOT EXISTS repository (
    id SERIAL PRIMARY KEY,
    owner TEXT NOT NULL,

    name TEXT NOT NULL,
    description TEXT,
    website TEXT,
    stars INTEGER NOT NULL,

    public_read BOOLEAN NOT NULL,
    public_write BOOLEAN NOT NULL,
    public_clone BOOLEAN NOT NULL,
    public_push BOOLEAN NOT NULL,
    public_chat BOOLEAN NOT NULL,

    chunk_count INTEGER NOT NULL,
    push_session_id TEXT,

    -- for easier search
    tags TEXT[] NOT NULL,

    created_at TIMESTAMPTZ NOT NULL,
    pushed_at TIMESTAMPTZ,
    search_index_built_at TIMESTAMPTZ,  -- if it's null, there's no search index
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS repository_by_owner ON repository ( owner, name );

CREATE TABLE IF NOT EXISTS repository_stat (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,

    date_str TEXT NOT NULL,  -- YYYYMMDD
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    day INTEGER NOT NULL,

    -- TODO: view count? but how?
    push INTEGER NOT NULL,
    clone INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS repository_stat_by_date ON repository_stat ( repo_id, date_str );

CREATE TABLE IF NOT EXISTS issue (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,
    ticket INTEGER NOT NULL,
    author TEXT NOT NULL,

    is_open BOOLEAN NOT NULL,
    is_deleted BOOLEAN NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_by_repo ON issue ( repo_id, ticket );
CREATE INDEX IF NOT EXISTS issue_by_open ON issue ( repo_id, is_open );

CREATE TABLE IF NOT EXISTS issue_content_history (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL,
    author TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_content_history_by_issue ON issue_content_history ( issue_id );

CREATE TABLE IF NOT EXISTS issue_comment (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL,
    author TEXT NOT NULL,

    is_deleted BOOLEAN NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_comment_by_issue ON issue_comment ( issue_id );
CREATE INDEX IF NOT EXISTS issue_comment_by_author ON issue_comment ( author );

CREATE TABLE IF NOT EXISTS issue_comment_content_history (
    id SERIAL PRIMARY KEY,
    issue_comment_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_comment_content_history_by_issue_comment ON issue_comment_content_history ( issue_comment_id );

CREATE TABLE IF NOT EXISTS chat (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS chat_by_repo ON chat ( repo_id );

CREATE TABLE IF NOT EXISTS chat_history (
    id SERIAL PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    turn INTEGER NOT NULL,

    -- any user can have a chat, so we need to store the user id
    user_ TEXT NOT NULL,
    model TEXT NOT NULL,

    query TEXT NOT NULL,
    response TEXT NOT NULL,
    multi_turn_schema TEXT,  -- json string

    created_at TIMESTAMPTZ NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS chat_history_by_chat ON chat_history ( chat_id, turn );

CREATE TABLE IF NOT EXISTS chat_history_chunk_uid (
    id SERIAL PRIMARY KEY,
    chat_history_id INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    chunk_uid TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS chat_history_chunk_uid_by_chat_history ON chat_history_chunk_uid ( chat_history_id, seq );

CREATE TABLE IF NOT EXISTS push_session (
    id TEXT PRIMARY KEY,
    repo_id INTEGER NOT NULL,

    -- going:      push operation is going on
    --             If a session is in `going` state and `updated_at` is more than 10 minutes ago,
    --             it'll be cleaned up and become `failed` state.
    --
    -- completed:  everything went well
    --
    -- failed:     `Index::extract_archive` function has failed.
    --             Its data is still on disk and has to be cleaned up.
    --
    -- failed_and_removed: It used to be at `failed` state, and the cleaner
    --                     removed its data from disk.
    --
    -- completed_and_removed: It used to be at `completed` state, but there's not enough
    --                        space in disk and the cleaner removed its data.
    session_state TEXT NOT NULL,
    updated_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS push_session_by_repo ON push_session ( repo_id, updated_at );
CREATE INDEX IF NOT EXISTS push_session_clean_up_index ON push_session ( session_state, updated_at );

CREATE TABLE IF NOT EXISTS archive (
    id SERIAL PRIMARY KEY,
    session_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    blob_size INTEGER NOT NULL,  -- in bytes
    blob_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS push_archive_by_session ON archive ( session_id, archive_id );

CREATE TABLE IF NOT EXISTS api_key (
    api_key TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    user_ TEXT NOT NULL,
    expire TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS api_key_by_user ON api_key ( user_ );
