-- Issue #31: la búsqueda de clientes hacía `lower(col) LIKE '%texto%'`, que
-- con un comodín al inicio nunca puede usar un índice B-tree normal (ni
-- `customers_name_idx` ni uno nuevo ayudarían) y termina en un full table
-- scan calculando `lower()` de 4 columnas en cada fila. FTS5 con el
-- tokenizador "trigram" sí permite que SQLite use un índice para acelerar
-- expresiones `LIKE '%texto%'` (no solo prefijos) sobre las columnas
-- indexadas — ver https://sqlite.org/fts5.html#the_trigram_tokenizer.
--
-- Se usa una tabla FTS5 "normal" (sin `content=`), no "external content":
-- guarda su propia copia en minúsculas de cada campo, sincronizada por
-- triggers con rowid explícito. Así los triggers son `INSERT`/`UPDATE`/
-- `DELETE` corrientes, sin la sintaxis especial (`INSERT INTO t(t, rowid,...)
-- VALUES('delete', ...)`) que solo hace falta con `content=` externo.

CREATE VIRTUAL TABLE customers_search USING fts5(
  name,
  email,
  document,
  phone,
  tokenize = 'trigram'
);

INSERT INTO customers_search(rowid, name, email, document, phone)
SELECT rowid, lower(name), lower(coalesce(email, '')), lower(coalesce(document, '')), lower(phone)
FROM customers;

CREATE TRIGGER customers_search_ai AFTER INSERT ON customers BEGIN
  INSERT INTO customers_search(rowid, name, email, document, phone)
  VALUES (
    new.rowid,
    lower(new.name),
    lower(coalesce(new.email, '')),
    lower(coalesce(new.document, '')),
    lower(new.phone)
  );
END;

CREATE TRIGGER customers_search_ad AFTER DELETE ON customers BEGIN
  DELETE FROM customers_search WHERE rowid = old.rowid;
END;

CREATE TRIGGER customers_search_au AFTER UPDATE ON customers BEGIN
  UPDATE customers_search SET
    name = lower(new.name),
    email = lower(coalesce(new.email, '')),
    document = lower(coalesce(new.document, '')),
    phone = lower(new.phone)
  WHERE rowid = new.rowid;
END;
