#!/bin/bash
BACKUP_DIR="./data/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

if [ -f "./data/trading.db" ]; then
    cp ./data/trading.db "$BACKUP_DIR/trading_$TIMESTAMP.db"
fi

find "$BACKUP_DIR" -type f -mtime +30 -delete

echo "✅ Backup terminé: $BACKUP_DIR"
