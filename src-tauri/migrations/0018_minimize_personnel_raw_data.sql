-- Personnel fields are already normalized; retaining a duplicate JSON row increases PII exposure.
UPDATE personnel SET raw_row_json = NULL;
