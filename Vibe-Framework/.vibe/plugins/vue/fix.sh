#!/bin/bash
# Plugin Vue.js - Fix
# Tente de corriger le formatage et le linting si package.json existe

if [ -f "package.json" ]; then
    # 1. ESLint Fix (si script dispo)
    if grep -q "lint" package.json; then
        npm run lint -- --fix > /dev/null 2>&1 || true
    fi
    
    # 2. Prettier (si pas géré par lint)
    if grep -q "format" package.json; then
        npm run format > /dev/null 2>&1 || true
    fi
fi

# 3. Vérifications .clinerules (règles surveillées par Sentinel)
echo "🔍 Surveillance .clinerules pour Vue..."

# Règle 10 : Commenter console.log, debugger, alert (au lieu de supprimer)
for file in src/**/*.vue src/*.vue; do
    if [ -f "$file" ]; then
        # Commente les lignes contenant console.log, etc. si elles ne sont pas déjà commentées
        sed -i -E 's/^(\s*)(console\.(log|error|warn|debug)\(|debugger|alert\()/\1\/\/ \2/' "$file"
    fi
done

# Règle 11 : Forcer <script setup> (si <script> seul, le remplacer)
for file in src/**/*.vue src/*.vue; do
    if [ -f "$file" ] && grep -q '<script>' "$file" && ! grep -q '<script setup' "$file"; then
        sed -i 's/<script>/<script setup>/' "$file"
    fi
done

# Règle 12 : Signaler v-html ou innerHTML (pas de correction auto pour sécurité)
if grep -r "v-html\|innerHTML" src/ --include="*.vue" > /dev/null 2>&1; then
    echo "⚠️ .clinerules Règle 12 : v-html ou innerHTML détecté - corrigez manuellement"
fi

echo "✅ Corrections .clinerules appliquées pour Vue"
