CREATE TABLE tasks (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0,
  priority INTEGER NOT NULL DEFAULT 3
);
INSERT INTO tasks (id, title, done, priority) VALUES
  (1, 'Buy milk',     0, 2),
  (2, 'Write report', 0, 1),
  (3, 'Call mom',     1, 3),
  (4, 'Pay rent',     0, 1),
  (5, 'Book flights', 1, 2);

CREATE TABLE counters (
  name TEXT PRIMARY KEY,
  value INTEGER NOT NULL
);
INSERT INTO counters VALUES ('visits', 10);
