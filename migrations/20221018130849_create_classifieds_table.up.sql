CREATE TABLE classifieds (
    session      uuid          NOT NULL,
    url          TEXT          NOT NULL,
    revision     SMALLINT      NOT NULL,
    extracted_at TIMESTAMPTZ   NOT NULL,

    PRIMARY KEY(session, url),
    CONSTRAINT fk_session_url_page
        FOREIGN KEY(session, url)
            REFERENCES pages(session, url)
            ON UPDATE CASCADE
);
