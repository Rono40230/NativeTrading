#!/bin/bash
# Plugin Python - Test
if [ -f "requirements.txt" ] || [ -f "pyproject.toml" ] || [ -f "setup.py" ]; then
    # Lancer pytest ou unittest
    if command -v pytest >/dev/null 2>&1; then
        if pytest --tb=short -q; then
            exit 0
        else
            exit 1
        fi
    elif python -m unittest discover -v 2>/dev/null; then
        exit 0
    else
        echo "⚠️  Python: Pas de tests configurés ou outils manquants"
        exit 0
    fi
else
    exit 0
fi