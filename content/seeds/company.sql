CREATE TABLE departments (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  city TEXT NOT NULL
);
CREATE TABLE employees (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  department_id INTEGER REFERENCES departments(id),
  manager_id INTEGER REFERENCES employees(id),
  salary INTEGER NOT NULL,
  hired_at TEXT NOT NULL
);

INSERT INTO departments VALUES
  (1, 'Engineering', 'Berlin'),
  (2, 'Sales',       'Madrid'),
  (3, 'Support',     'Lisbon'),
  (4, 'Legal',       'Paris');

INSERT INTO employees VALUES
  (1,  'Alice', 1,    NULL, 9000, '2018-03-01'),
  (2,  'Bruno', 1,    1,    7000, '2019-06-15'),
  (3,  'Carla', 1,    2,    6000, '2020-09-01'),
  (4,  'Diego', 1,    2,    5500, '2021-01-10'),
  (5,  'Elena', 2,    1,    6500, '2019-02-20'),
  (6,  'Farid', 2,    5,    4800, '2022-05-05'),
  (7,  'Gina',  2,    5,    4800, '2022-11-11'),
  (8,  'Hugo',  3,    1,    4200, '2021-07-19'),
  (9,  'Iris',  3,    8,    3900, '2023-02-01'),
  (10, 'Jonas', NULL, 1,    5000, '2020-12-12');
