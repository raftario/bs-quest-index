CREATE TABLE IF NOT EXISTS mods (
    id varchar(64) NOT NULL,

    major int NOT NULL,
    minor int NOT NULL,
    patch int NOT NULL,

    UNIQUE(id, major, minor, patch)
)
