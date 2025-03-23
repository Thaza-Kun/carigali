-- Add up migration script here
CREATE TABLE doc_info (
    document text not null primary key,
    term_count integer not null
);

CREATE TRIGGER update_doc_info AFTER INSERT ON term_info BEGIN
INSERT INTO
    doc_info (term_count, document)
VALUES
    (1, new.document) ON CONFLICT (document) DO
UPDATE
SET
    term_count = term_count + new.occurence;

UPDATE term_info
SET
    frequency = (
        SELECT
            cast(occurence as real) / cast(term_count as real)
        FROM
            term_info
            JOIN doc_info ON term_info.document = doc_info.document
        WHERE
            term_info.document = new.document
    )
WHERE
    term_info.document = new.document;

END;
