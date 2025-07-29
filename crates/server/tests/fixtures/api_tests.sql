-- Test user 1: password is 'password123'
INSERT INTO users (id, username, name, password, enabled, admin, "limit", created_at, created_by)
  VALUES (
    '184f75f5-d345-4aae-92da-83c853125793', 'user1', 'User 1',
    '$argon2id$v=19$m=20480,t=2,p=1$J4vIwv/qkliQHossNbUhiQ$Ma+2B8Ydw+fB7dtjL2Y2hUdS41VzhCw9QTB9Sw3EZ50',
    1, 0, NULL,
    "2025-01-01T00:00:00Z", NULL
  );

INSERT INTO api_keys (id, owner, code, name, enabled, created_at, created_by)
  VALUES (
    '59d114ef-9de1-409c-8b31-caf7fa8bf851',
    '184f75f5-d345-4aae-92da-83c853125793',
    'testapikey1234567890',
    'Test API Key 1',
    1,
    "2025-01-01T00:00:00Z",
    '184f75f5-d345-4aae-92da-83c853125793'
  );

INSERT INTO uploads (id, owner_user, slug, filename, size, public, downloads, uploaded_at, has_preview)
  VALUES (
    'c968840a-79d0-4b95-a36e-2cd95e4773ac', '184f75f5-d345-4aae-92da-83c853125793',
    'ywAjcpQi', 'testfile1.txt', 1024, 0, 0, '2025-01-01T00:00:00Z', 0
  ), (
    '76da2990-4d85-48ba-9931-e95581eb8ed2', '184f75f5-d345-4aae-92da-83c853125793',
    'H7rmeHqJ', 'testfile2.txt', 2048, 0, 0, '2025-01-02T00:00:00Z', 0
  );
