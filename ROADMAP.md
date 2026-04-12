# 🗺️ ROADMAP — Native Trading AI
> Dernière mise à jour : 8 avril 2026

## � RÈGLES ABSOLUES (non négociables)

### Règle 1 — Zéro régression
Avant toute implémentation, analyser les conséquences sur le code existant :
- Lister les fichiers impactés (directs et indirects)
- Identifier les appels entrants vers les fonctions modifiées
- Vérifier que les signatures d'API REST ne changent pas sans migration
- Lancer `cargo build --workspace` + `cargo test --workspace` après chaque modification
- Si un test existant échoue → corriger AVANT de continuer, jamais ignorer

### Règle 2 — Explication avant action
Pour chaque tâche non triviale, fournir AVANT de toucher au code :
1. **Ce qui va être modifié** : liste des fichiers + fonctions concernées
2. **Pourquoi** : justification métier ou technique
3. **Risques identifiés** : ce qui pourrait casser
4. **Plan de rollback** : comment revenir si ça échoue
→ L'utilisateur valide (explicitement ou par silence >30s) avant l'implémentation.

---

### Règle 3 - WORKFLOW (rappel)

Chaque item coché = audit obligatoire avant commit :
```bash
./.vibe/bin/audit.sh        # Clippy + tests + taille fichiers + zero-unwrap
cargo test --workspace      # Tous les tests backend
cd frontend && npm run test # Tests Vue
```

Signal d'alerte :
- Fichier ≥ 250 lignes → split immédiat
- `unwrap()` / `console.log()` → bloquant
- Calcul métier côté Vue → interdit (tout passe par le backend)

---

## 🚀 PLAN — 