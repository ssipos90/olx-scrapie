CREATE TABLE pages (
    content      TEXT          NOT NULL,
    crawled_at   TIMESTAMPTZ   NOT NULL,
    page_type    page_type     NOT NULL,
    session      uuid          NOT NULL,
    url          TEXT          NOT NULL,

    PRIMARY KEY(session, url)
);
