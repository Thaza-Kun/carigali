-- SPACE TO SAVE EXPERIMENTED QUERIES

-- COUNT UNIQUE ITEMS
-- <counting.sql>
SELECT 
    COUNT(DISTINCT document) as documents, 
    -- `lower` is normalized `terms` 
    COUNT(DISTINCT lower) as unique_tokens, 
    SUM(occurence) as token_total
FROM term_frequency

-- TOKEN PER DOCUMENT
-- <docinfo.sql>
SELECT
    SUM(occurence) as token_total,
    document
FROM term_frequency
GROUP BY document

-- TERM-FREQUENCY
-- <tf.sql>
WITH doc AS (
    -- refer: <docinfo.sql>
    SELECT SUM(occurence) as tokens, document FROM term_frequency GROUP BY document
    )
SELECT 
    occurence/cast(tokens as real) as frequency, 
    lower, 
    term_frequency.document 
FROM term_frequency 
JOIN doc ON doc.document = term_frequency.document

-- NUMBER OF TIMES TERM APPEAR IN DOC
-- <docfreq.sql>
SELECT 
    COUNT(DISTINCT document) as docfreq, 
    lower 
FROM term_frequency 
GROUP BY lower

-- TF-IDF
WITH counting AS (
    -- REFER: <counting.sql>
    SELECT
        COUNT(DISTINCT document) as documents,
        COUNT(DISTINCT lower) as unique_tokens,
        SUM(occurence) as token_total
    FROM term_frequency
    ),
    docinfo as (
    -- REFER: <docinfo.sql>
    SELECT
        SUM(occurence) as token_total,
        document
    FROM term_frequency
    GROUP BY document
    ),
    terminfo AS (
    -- REFER: <tf.sql>
    SELECT
        occurence/cast(token_total as real) as frequency,
        lower,
        term_frequency.document
    FROM term_frequency
    JOIN docinfo ON docinfo.document = term_frequency.document
    ),
    docfreq AS (
    -- REFER: <docfreq.sql>
    SELECT
        COUNT(DISTINCT document) as docfreq,
        counting.documents as doctotal,
        lower
    FROM term_frequency
    JOIN counting
    GROUP BY lower
    )
SELECT 
    -- NO NATIVE LOG10 FUNCTION SO this is equivalent to 10^{idf} <= N/DF
    doctotal/cast(docfreq as real) as power_of_idf,
    frequency as tf, 
    docfreq.lower 
FROM docfreq 
JOIN terminfo on terminfo.lower = docfreq.lower
ORDER BY docfreq.docfreq

-- Aggregate TFIDF directly in SQL
WITH counting AS (
    -- REFER: <counting.sql>
    SELECT
        COUNT(DISTINCT document) as documents,
        COUNT(DISTINCT lower) as unique_tokens,
        SUM(occurence) as token_total
    FROM term_frequency
    ),
    docinfo as (
    -- REFER: <docinfo.sql>
    SELECT
        SUM(occurence) as token_total,
        document
    FROM term_frequency
    GROUP BY document
    ),
    terminfo AS (
    -- REFER: <tf.sql>
    SELECT
        occurence/cast(token_total as real) as frequency,
        lower,
        term_frequency.document
    FROM term_frequency
    JOIN docinfo ON docinfo.document = term_frequency.document
    ),
    docfreq AS (
    -- REFER: <docfreq.sql>
    SELECT
        COUNT(DISTINCT document) as docfreq,
        counting.documents as doctotal,
        lower
    FROM term_frequency
    JOIN counting
    GROUP BY lower
    ),
    final as (SELECT
        -- Approximates log10 
        2*(
            ((doctotal/cast(docfreq as real))/1)
            +(
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))/3
            )
            +(
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))/5
            )
            +(
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))*
                (doctotal/cast(docfreq as real))/7
            )
        ) as idf,
        frequency as tf,
        docfreq.lower
    FROM docfreq
    JOIN terminfo on terminfo.lower = docfreq.lower
    ORDER BY docfreq.docfreq
    )
SELECT MAX(tf*idf) as maxtfidf, AVG(tf*idf) as meantfidf, MIN(tf*idf) as mintfidf, lower FROM final GROUP BY lower ORDER BY maxtfidf DESC