CREATE TABLE pages (
    added_at  TIMESTAMPTZ   NOT NULL,
    session   uuid          NOT NULL,
    url       TEXT          NOT NULL,
    page_type page_type     NOT NULL,
    content   TEXT,

    PRIMARY KEY(session, url)
);
