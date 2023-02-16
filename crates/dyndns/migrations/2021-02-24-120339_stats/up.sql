-- Generic key value store.
CREATE TABLE info (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL
);

-- Domains.
CREATE TABLE domain_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name VARCHAR NOT NULL,
    record_type VARCHAR NOT NULL
);

-- Stores each fetch event. Fact table.
CREATE TABLE domain_ip_fetches (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    attempt_date DATETIME NOT NULL,
    success BOOLEAN NOT NULL,
    fetched_ipv4 VARCHAR,
    fetched_ipv6 VARCHAR
);

-- Stores last ip fetch event. Can be recomputed.
CREATE TABLE domain_ip_last_fetches (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    attempt_count INTEGER DEFAULT 0 NOT NULL,
    success_count INTEGER DEFAULT 0 NOT NULL,
    fail_count INTEGER DEFAULT 0 NOT NULL,
    last_attempt_date DATETIME NOT NULL,
    last_success_date DATETIME,
    last_successful_fetched_ipv4 VARCHAR,
    last_successful_fetched_ipv4_change_date DATETIME NOT NULL,
    last_successful_fetched_ipv6 VARCHAR,
    last_successful_fetched_ipv6_change_date DATETIME NOT NULL
);

--  Stores each record update event. Fact table.
CREATE TABLE domain_record_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    domain_record_id INTEGER NOT NULL REFERENCES domain_records(id),
    set_ip VARCHAR NOT NULL,
    attempt_date DATETIME NOT NULL,
    success BOOLEAN NOT NULL
);

-- Stores last record update event for each domain id. Can be recomputed.
CREATE TABLE domain_record_last_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    domain_record_id INTEGER UNIQUE NOT NULL REFERENCES domain_records(id),
    attempt_count INTEGER DEFAULT 0 NOT NULL,
    success_count INTEGER DEFAULT 0 NOT NULL,
    fail_count INTEGER DEFAULT 0 NOT NULL,
    last_attempt_date DATETIME NOT NULL,
    last_success_date DATETIME,
    last_set_ip VARCHAR NOT NULL
);

--  Stores each updater attempt. Fact table.
CREATE TABLE updater_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    domain_record_id INTEGER REFERENCES domain_records(id) NOT NULL,
    domain_ip_fetches_id INTEGER NOT NULL REFERENCES domain_ip_fetches(id),
    domain_record_updates_id INTEGER REFERENCES domain_record_updates(id),
    attempt_date DATETIME NOT NULL

    -- ip VARCHAR,
    -- last_updater_run_date DATETIME NOT NULL,
    -- last_ip_fetch_date DATETIME,
    -- last_ip_change_date DATETIME,
    --last_domain_record_update_date DATETIME
);

-- TODO: Come up with recomputable attempts table.
-- ip -> Nullable<Text>,
-- last_updater_run_date -> Timestamp,
-- last_ip_fetch_date -> Nullable<Timestamp>,
-- last_ip_change_date -> Nullable<Timestamp>,
-- last_domain_record_update_date -> Nullable<Timestamp>,
