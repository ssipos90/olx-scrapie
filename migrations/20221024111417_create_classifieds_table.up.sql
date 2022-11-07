CREATE TABLE classifieds (
    session      uuid          NOT NULL,
    url          TEXT          NOT NULL,
    revision     SMALLINT      NOT NULL,
    extracted_at TIMESTAMPTZ   NOT NULL,

    orientation cardinal_direction,
    floor smallint,
    layout property_layout,
    negotiable boolean,
    price float NOT NULL,
    property_type property_type NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    room_count smallint,
    seller_name TEXT NOT NULL,
    seller_type seller_type NOT NULL,
    surface integer,
    title TEXT NOT NULL,
    year integer,

    PRIMARY KEY(session, url),
    CONSTRAINT fk_session_url_page
        FOREIGN KEY(session, url)
            REFERENCES pages(session, url)
            ON UPDATE CASCADE
);
