-- Add up migration script here
CREATE TABLE IF NOT EXISTS term_info (
    document text not null,
    term text not null,
    lower text not null,
    occurence integer not null, -- not null ensures proper triggers and conflict resolution
    frequency real,
    primary key (document, term, lower)
);

CREATE TABLE IF NOT EXISTS term_doc_info (
    term text not null primary key,
    doc_count integer not null
);

CREATE TRIGGER IF NOT EXISTS update_term_doc_info AFTER INSERT ON term_info
BEGIN
    INSERT INTO term_doc_info (term, doc_count) VALUES (new.lower, 1)
    ON CONFLICT (term) DO UPDATE SET doc_count = doc_count + 1;
END;

CREATE TABLE IF NOT EXISTS ngram_two (
    document text not null,
    term text not null,
    lower1 text not null,
    lower2 text not null,
    occurence integer not null
);

CREATE TABLE IF NOT EXISTS ngram_three (
    document text not null,
    term text not null,
    lower1 text not null,
    lower2 text not null,
    lower3 text not null,
    occurence integer not null
)