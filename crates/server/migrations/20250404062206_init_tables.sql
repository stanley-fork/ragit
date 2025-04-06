-- NOTE: postgresql doesn't allow me to use `user` as a name of a table
CREATE TABLE IF NOT EXISTS user_ (
    id SERIAL PRIMARY KEY,

    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    email TEXT,
    salt TEXT NOT NULL,
    password TEXT NOT NULL,
    password_hash_type TEXT NOT NULL,
    readme TEXT,
    public BOOLEAN NOT NULL,

    created_at TIMESTAMPTZ NOT NULL,
    last_login_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX IF NOT EXISTS user_by_name ON user_ ( normalized_name );

CREATE TABLE IF NOT EXISTS repository (
    id SERIAL PRIMARY KEY,
    owner_id INTEGER NOT NULL,

    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    description TEXT,
    website TEXT,
    stars INTEGER NOT NULL,
    readme TEXT,

    public_read BOOLEAN NOT NULL,
    public_write BOOLEAN NOT NULL,
    public_clone BOOLEAN NOT NULL,
    public_push BOOLEAN NOT NULL,

    chunk_count INTEGER NOT NULL,
    repo_size BIGINT NOT NULL,  -- in bytes (sum of the sizes of the archive files)
    push_session_id TEXT,

    created_at TIMESTAMPTZ NOT NULL,
    pushed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS repository_by_owner ON repository ( owner_id );

CREATE TABLE IF NOT EXISTS push_clone (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,

    date_str TEXT NOT NULL,  -- YYYYMMDD
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    day INTEGER NOT NULL,

    push INTEGER NOT NULL,
    clone INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS push_clone_by_date ON push_clone ( repo_id, date_str );

CREATE TABLE IF NOT EXISTS issue (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,
    ticket INTEGER NOT NULL,
    author_id INTEGER NOT NULL,

    is_open BOOLEAN NOT NULL,
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
    author_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_content_history_by_issue ON issue_content_history ( issue_id );

CREATE TABLE IF NOT EXISTS issue_comment (
    id SERIAL PRIMARY KEY,
    issue_id INTEGER NOT NULL,
    author_id INTEGER NOT NULL,

    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS issue_comment_by_issue ON issue_comment ( issue_id );
CREATE INDEX IF NOT EXISTS issue_comment_by_author ON issue_comment ( author_id );

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
    user_id INTEGER NOT NULL,
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

CREATE TABLE IF NOT EXISTS push_archive (
    id SERIAL PRIMARY KEY,
    session_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    blob_size INTEGER NOT NULL,  -- in bytes
    blob_id TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS push_archive_by_session ON push_archive ( session_id );

CREATE TABLE IF NOT EXISTS push_blob (
    id TEXT PRIMARY KEY,
    blob BYTEA
);
