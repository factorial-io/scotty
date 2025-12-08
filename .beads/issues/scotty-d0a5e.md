---
title: Add documentation and examples for file transfer features
status: open
priority: 3
issue_type: task
depends_on:
  scotty-1836f: parent-child
  scotty-f4e02: blocks
  scotty-0fb7b: blocks
created_at: 2025-12-08T23:54:43.461472+00:00
updated_at: 2025-12-08T23:54:43.461472+00:00
---

# Description

Document both stdin piping and app:cp command with real-world examples.

**Documentation locations**:
- docs/content/cli.md: Add file transfer section
- README.md: Add file transfer examples
- scottyctl --help: Ensure good help text for app:cp

**Example workflows to document**:

1. Database backup/restore:
```bash
## Using pipes
scottyctl app:shell db mysql -c "mysqldump mydb | gzip" > backup.sql.gz
zcat backup.sql.gz | scottyctl app:shell db mysql -c "mysql mydb"

## Using app:cp
scottyctl app:cp db:mysql:/var/lib/mysql ./mysql-backup
scottyctl app:cp ./mysql-backup db:mysql:/var/lib/mysql
```

2. Log collection:
```bash
scottyctl app:cp myapp:web:/var/log/nginx ./nginx-logs
scottyctl app:shell myapp web -c "tar -czf - /var/log" > logs.tar.gz
```

3. Configuration deployment:
```bash
scottyctl app:cp ./nginx.conf myapp:web:/etc/nginx/nginx.conf
scottyctl app:shell myapp web -c "nginx -s reload"
```

4. Asset upload:
```bash
scottyctl app:cp ./product-images myapp:web:/var/www/public/images
```

**Comparison table**: When to use pipes vs app:cp

**Time estimate**: 2 hours
