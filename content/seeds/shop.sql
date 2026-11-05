CREATE TABLE customers (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT,
  city TEXT NOT NULL,
  country TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE products (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL,
  price REAL NOT NULL,
  stock INTEGER NOT NULL
);
CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  customer_id INTEGER NOT NULL REFERENCES customers(id),
  ordered_at TEXT NOT NULL,
  status TEXT NOT NULL
);
CREATE TABLE order_items (
  order_id INTEGER NOT NULL REFERENCES orders(id),
  product_id INTEGER NOT NULL REFERENCES products(id),
  quantity INTEGER NOT NULL,
  PRIMARY KEY (order_id, product_id)
);

INSERT INTO customers VALUES
  (1, 'Ada Lovelace',      'ada@example.com',      'London',      'UK',     '2023-01-15'),
  (2, 'Grace Hopper',      'grace@example.com',    'New York',    'USA',    '2023-02-20'),
  (3, 'Alan Turing',       NULL,                   'London',      'UK',     '2023-03-05'),
  (4, 'Linus Torvalds',    'linus@example.com',    'Portland',    'USA',    '2023-04-11'),
  (5, 'Margaret Hamilton', 'margaret@example.com', 'Boston',      'USA',    '2023-05-30'),
  (6, 'Dennis Ritchie',    NULL,                   'Newark',      'USA',    '2023-06-18'),
  (7, 'Barbara Liskov',    'barbara@example.com',  'Los Angeles', 'USA',    '2023-07-02'),
  (8, 'Tim Berners-Lee',   'tim@example.com',      'London',      'UK',     '2023-08-09'),
  (9, 'Marie Curie',       'marie@example.com',    'Paris',       'France', '2023-09-21');

INSERT INTO products VALUES
  (1,  'Laptop',     'electronics', 1200.00,  15),
  (2,  'Phone',      'electronics',  800.00,  30),
  (3,  'Headphones', 'electronics',  150.00,   0),
  (4,  'Desk',       'furniture',    300.00,   8),
  (5,  'Chair',      'furniture',    120.00,  25),
  (6,  'Notebook',   'stationery',     3.50, 200),
  (7,  'Pen',        'stationery',     1.20, 500),
  (8,  'Backpack',   'accessories',   45.00,  40),
  (9,  'Monitor',    'electronics',  250.00,  12),
  (10, 'Lamp',       'furniture',     35.00,  60);

INSERT INTO orders VALUES
  (1,  1, '2024-01-05', 'shipped'),
  (2,  1, '2024-02-14', 'shipped'),
  (3,  2, '2024-01-20', 'shipped'),
  (4,  2, '2024-03-02', 'cancelled'),
  (5,  4, '2024-02-01', 'shipped'),
  (6,  5, '2024-02-10', 'pending'),
  (7,  5, '2024-03-15', 'shipped'),
  (8,  7, '2024-03-20', 'shipped'),
  (9,  8, '2024-03-25', 'pending'),
  (10, 1, '2024-04-02', 'pending');

INSERT INTO order_items VALUES
  (1, 1, 1), (1, 6, 10),
  (2, 3, 2),
  (3, 2, 1), (3, 8, 1),
  (4, 4, 1),
  (5, 5, 2), (5, 9, 1),
  (6, 6, 20), (6, 7, 50),
  (7, 1, 1), (7, 9, 2),
  (8, 2, 2),
  (9, 8, 3),
  (10, 5, 1), (10, 6, 5);
