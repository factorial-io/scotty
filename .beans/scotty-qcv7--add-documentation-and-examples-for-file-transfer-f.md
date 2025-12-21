---
# scotty-qcv7
title: Add documentation and examples for file transfer features
status: todo
type: task
priority: normal
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-fad6
---

# Description  Document both stdin piping and app:cp command with real-world examples.  **Documentation locations**: - docs/content/cli.md: Add file transfer section - README.md: Add file transfer examples - scottyctl --help: Ensure good help text for app:cp  **Example workflows to document**:  1. Database backup/restore: ```bash ## Using pipes scottyctl app:shell db mysql -c "mysqldump mydb | gzip" > backup.sql.gz zcat backup.sql.gz | scottyctl app:shell db mysql -c "mysql mydb"  ## Using app:cp scottyctl app:cp db:mysql:/var/lib/mysql ./mysql-backup scottyctl app:cp ./mysql-backup db:mysql:/var/lib/mysql ```  2. Log collection: ```bash scottyctl app:cp myapp:web:/var/log/nginx ./nginx-logs scottyctl app:shell myapp web -c "tar -czf - /var/log" > logs.tar.gz ```  3. Configuration deployment: ```bash scottyctl app:cp ./nginx.conf myapp:web:/etc/nginx/nginx.conf scottyctl app:shell myapp web -c "nginx -s reload" ```  4. Asset upload: ```bash scottyctl app:cp ./product-images myapp:web:/var/www/public/images ```  **Comparison table**: When to use pipes vs app:cp  **Time estimate**: 2 hours
