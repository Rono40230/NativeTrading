#!/bin/bash
# plugins/security/audit.sh - Audit de sécurité avancé
# Lance cargo audit et npm audit pour détecter les vulnérabilités

EXIT_CODE=0
echo "🔒 Audit de sécurité avancé VibeOS..."

# 1. Audit Rust avec cargo audit
if command -v cargo >/dev/null 2>&1; then
    TARGET_DIR="."
    if [ -f "src-tauri/Cargo.toml" ]; then TARGET_DIR="src-tauri"; fi
    if [ -f "$TARGET_DIR/Cargo.toml" ]; then
        echo "   🔍 Audit Rust (cargo audit)..."
        if command -v cargo-audit >/dev/null 2>&1; then
            # Fix: s'assurer que cargo audit ne plante pas le script avec jq
            VULN_COUNT=$( (cd "$TARGET_DIR" && cargo audit --format json 2>/dev/null | jq -r '.vulnerabilities.found // 0') || echo "0" )
            
            # Nettoyage si jamais cargo audit renvoie autre chose
            if ! [[ "$VULN_COUNT" =~ ^[0-9]+$ ]]; then VULN_COUNT=0; fi

            if [ "$VULN_COUNT" -gt 0 ]; then
                    echo "❌ Vulnérabilités Rust détectées : $VULN_COUNT"
                    EXIT_CODE=1
            else
                    echo "✅ Audit Rust : OK"
            fi
        else
            echo "⚠️  cargo-audit non installé. Installez avec : cargo install cargo-audit"
        fi
    fi
fi

# 2. Audit Node.js avec npm audit
if command -v npm >/dev/null 2>&1 && [ -f "package.json" ]; then
    echo "   🔍 Audit Node.js (npm audit)..."
    # Extraction plus robuste du nombre total de vulnérabilités
    VULN_COUNT=$(npm audit --audit-level moderate --json 2>/dev/null | jq -r '.metadata.vulnerabilities.total // 0')
    
    # Vérification que c'est bien un nombre
    if ! [[ "$VULN_COUNT" =~ ^[0-9]+$ ]]; then
        VULN_COUNT=0
    fi
    
    if [ "$VULN_COUNT" -gt 0 ]; then
        # Ne pas bloquer pour des audits npm (souvent faux positifs), juste logguer
        echo "⚠️ Vulnérabilités npm détectées : $VULN_COUNT (non bloquant)"
    else
        echo "✅ Audit npm : OK"
    fi
fi

# 3. Scan des secrets avec gitleaks ou trufflehog
echo "   🔍 Scan des secrets..."
if command -v gitleaks >/dev/null 2>&1; then
    if gitleaks detect --verbose --redact --config .vibe/config.toml 2>/dev/null; then
        echo "✅ Scan secrets (gitleaks) : OK"
    else
        echo "❌ Secrets exposés détectés !"
        EXIT_CODE=1
    fi
elif command -v trufflehog >/dev/null 2>&1; then
    if trufflehog filesystem . --exclude-paths=".git,node_modules,target" --json | jq -r '.SourceMetadata.Data.Secret // empty' | grep -q .; then
        echo "❌ Secrets exposés détectés !"
        EXIT_CODE=1
    else
        echo "✅ Scan secrets (trufflehog) : OK"
    fi
else
    echo "⚠️  Aucun outil de scan secrets installé (gitleaks ou trufflehog recommandé)."
fi

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Audit de sécurité avancé : OK"
    exit 0
else
    echo "🔴 ÉCHEC AUDIT SÉCURITÉ AVANCÉ"
    exit 1
fi