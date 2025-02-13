-- Add down migration script here
DROP TRIGGER update_term_doc_info;
DROP TABLE term_info;
DROP TABLE term_doc_info;
DROP TABLE ngram_two;
DROP TABLE ngram_three;