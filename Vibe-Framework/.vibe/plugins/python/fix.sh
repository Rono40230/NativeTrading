#!/bin/bash
# Plugin Python - Fix
if [ -f "requirements.txt" ] || [ -f "pyproject.toml" ] || [ -f "setup.py" ]; then
    # Formatage avec black et isort si disponibles
    if command -v black >/dev/null 2>&1; then
        black . --quiet
    fi
    if command -v isort >/dev/null 2>&1; then
        isort . --quiet
    fi
    # Linting avec flake8 ou pylint
    if command -v flake8 >/dev/null 2>&1; then
        flake8 --max-line-length=88 --extend-ignore=E203,W503 . || true  # Ne bloque pas
    fi
fi