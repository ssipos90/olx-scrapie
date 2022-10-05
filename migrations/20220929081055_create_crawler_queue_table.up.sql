CREATE TABLE crawler_queue (
    status crawl_status NOT NULL DEFAULT 'new',
    session uuid NOT NULL,
    url TEXT NOT NULL,
    page_type page_type NOT NULL,
    added_at TIMESTAMPTZ NOT NULL,
    not_before TIMESTAMPTZ NOT NULL,
    retries TEXT[],

    PRIMARY KEY(session, url)
)
