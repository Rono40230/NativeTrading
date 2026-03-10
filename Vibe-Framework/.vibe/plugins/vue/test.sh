#!/bin/bash
# Plugin Vue.js - Test

if [ -f "package.json" ]; then
    # Vérifie si le script de test existe
    if grep -q "test:unit" package.json; then
        if npm run test:unit -- --run; then
            exit 0
        else
            exit 1
        fi
    elif grep -q "\"test\"" package.json; then
        if npm run test; then
            exit 0
        else
             exit 1
        fi
    else
        # Pas de tests configurés, on considère que c'est OK (ou warning)
        echo "⚠️  Vue: Pas de script de test trouvé"
        exit 0
    fi

    # Lancer E2E si disponible
    if [ -f ".vibe/plugins/vue/e2e.sh" ]; then
        ./.vibe/plugins/vue/e2e.sh || exit 1
    fi
else
    exit 0
fi
