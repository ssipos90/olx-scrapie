CREATE TABLE sessions (
    session uuid NOT NULL,
    completed BOOL NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY(session)
);
