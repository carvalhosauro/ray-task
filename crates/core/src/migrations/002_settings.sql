-- Preferências do app (chave/valor). Valores inválidos voltam ao padrão na leitura.
CREATE TABLE settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL
);
