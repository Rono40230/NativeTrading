#!/bin/bash
# plugins/vue/e2e.sh - Tests End-to-End pour Vue.js
# Lance Playwright ou Cypress si configuré

if [ -f "package.json" ]; then
    # Vérifier si e2e est configuré
    if grep -q "test:e2e" package.json; then
        echo "🚀 Lancement des tests E2E (Playwright/Cypress)..."
        if npm run test:e2e -- --run; then
            echo "✅ Tests E2E réussis"
            exit 0
        else
            echo "❌ Échec tests E2E"
            exit 1
        fi
    elif grep -q "cypress" package.json || grep -q "playwright" package.json; then
        # Essayer de lancer directement
        if command -v npx >/dev/null 2>&1; then
            if npx playwright test --headed=false 2>/dev/null; then
                echo "✅ Tests E2E (Playwright) réussis"
                exit 0
            elif npx cypress run 2>/dev/null; then
                echo "✅ Tests E2E (Cypress) réussis"
                exit 0
            else
                echo "❌ Aucun outil E2E trouvé ou configuré"
                exit 1
            fi
        else
            echo "⚠️  npx non disponible pour lancer E2E"
            exit 0  # Ne bloque pas si pas configuré
        fi
    else
        echo "ℹ️  Pas de tests E2E configurés"
        exit 0
    fi
else
    exit 0
fi