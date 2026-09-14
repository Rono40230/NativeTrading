#!/usr/bin/env python3
"""Diff 3 voies KDJ/Halftrend — rejeu Rust ↔ EA MQ5 (au trade près).

Usage :
    python3 scripts/diff_kdj_3voies.py rust.csv ea.csv [--tol-prix 0.01] [--skip N]

Les deux CSV partagent les colonnes produites par `replay_kdj --dump` et par
l'EA (`kdj_diag_<symbole>_<tf>.csv` dans MQL5/Files/) :
    ts_entree,dir,open_entree,fill_entree,sl_niveau,tp_niveau,
    ts_sortie,open_sortie,fill_sortie,verdict

Matching par (ts_entree, dir). Tolérance de prix absolue (--tol-prix) pour
open/niveaux/sortie : au centime par défaut. --skip N ignore les N premiers
trades communs (divergences de seed EMA/état sur le début de fenêtre).
Sortie : rapport détaillé ; exit 1 si divergence, 0 si parité.
"""
import argparse
import csv
import sys


def charger(chemin: str, decalage_s: int = 0) -> dict:
    trades = {}
    with open(chemin, newline="", encoding="utf-8-sig") as f:
        for ligne in csv.DictReader(f):
            cle = (int(ligne["ts_entree"]) + decalage_s, int(ligne["dir"]))
            trades[cle] = {
                "open_entree": float(ligne["open_entree"]),
                "sl_niveau": float(ligne["sl_niveau"]),
                "tp_niveau": float(ligne["tp_niveau"]),
                "ts_sortie": int(ligne["ts_sortie"] or 0) + decalage_s,
                "open_sortie": float(ligne["open_sortie"] or 0.0),
                "verdict": ligne["verdict"].strip(),
            }
    return trades


def detecter_decalage(rust: dict, ea_brut: str) -> int:
    """Le broker MT5 horodate en heure serveur (ex. Axi UTC+2/+3), la base en
    UTC : on cherche le décalage horaire entier qui maximise les matches."""
    meilleur, meilleure_cle = 0, len(set(rust) & set(charger(ea_brut)))
    for h in range(-13, 14):
        if h == 0:
            continue
        n = len(set(rust) & set(charger(ea_brut, h * 3600)))
        if n > meilleure_cle:
            meilleur, meilleure_cle = h * 3600, n
    return meilleur


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("rust")
    p.add_argument("ea")
    p.add_argument("--tol-prix", type=float, default=0.01)
    p.add_argument("--skip", type=int, default=0)
    args = p.parse_args()

    decalage = detecter_decalage(charger(args.rust), args.ea)
    if decalage:
        print(f"Décalage horaire détecté : {decalage // 3600:+d} h "
              f"(heure serveur broker vs UTC) — appliqué aux ts EA.")
    rust, ea = charger(args.rust), charger(args.ea, decalage)
    print(f"Rust : {len(rust)} trades · EA : {len(ea)} trades")

    cles_communes = sorted(set(rust) & set(ea))[args.skip:]
    ecarts = 0
    for cle in sorted(set(rust) & set(ea))[:args.skip]:
        print(f"  (skip) {cle} — exclu (seed fenêtre)")

    for cle in cles_communes:
        r, e = rust[cle], ea[cle]
        diffs = []
        for champ in ("open_entree", "sl_niveau", "tp_niveau", "open_sortie"):
            if abs(r[champ] - e[champ]) > args.tol_prix:
                diffs.append(f"{champ}: rust {r[champ]:.5f} vs ea {e[champ]:.5f}")
        if r["ts_sortie"] != e["ts_sortie"]:
            diffs.append(f"ts_sortie: rust {r['ts_sortie']} vs ea {e['ts_sortie']}")
        if r["verdict"] != e["verdict"]:
            diffs.append(f"verdict: rust {r['verdict']} vs ea {e['verdict']}")
        if diffs:
            ecarts += 1
            print(f"  ✗ {cle[0]} dir {cle[1]:+d} : " + " · ".join(diffs))

    only_rust = sorted(set(rust) - set(ea))
    only_ea = sorted(set(ea) - set(rust))
    # Trades Rust hors de la fenêtre temporelle du test EA : attendus (le
    # backtest MT5 ne couvre que quelques mois, le dump Rust toute la fenêtre
    # chargée). Bornes = entrées du CSV EA, matchées ou non.
    if ea:
        fen_min = min(cle[0] for cle in ea)
        fen_max = max(cle[0] for cle in ea)
        hors = [cle for cle in only_rust if cle[0] < fen_min - 7 * 86400 or cle[0] > fen_max + 7 * 86400]
        if hors:
            print(f"  ({len(hors)} trades Rust hors fenêtre du test EA — non comptés)")
            only_rust = [cle for cle in only_rust if cle not in hors]
    # Rattrapage DST : le broker passe UTC+3 → UTC+2 dans la période ; les
    # trades après le changement restent décalés d'une heure du décalage
    # principal. On apparie les rescapés à ±2 h avec même direction.
    dst_matches = []
    ea_restes = list(only_ea)
    for cr in list(only_rust):
        for ce in list(ea_restes):
            if ce[1] == cr[1] and abs(ce[0] - cr[0]) <= 2 * 3600:
                dst_matches.append((cr, ce))
                only_rust.remove(cr)
                ea_restes.remove(ce)
                break
    only_ea = ea_restes
    for cr, ce in dst_matches:
        r, e = rust[cr], ea[ce]
        diffs = []
        for champ in ("open_entree", "sl_niveau", "tp_niveau", "open_sortie"):
            if abs(r[champ] - e[champ]) > args.tol_prix:
                diffs.append(f"{champ}: rust {r[champ]:.5f} vs ea {e[champ]:.5f}")
        if r["verdict"] != e["verdict"]:
            diffs.append(f"verdict: {r['verdict']} vs ea {e['verdict']}")
        if diffs:
            ecarts += 1
            print(f"  ✗ {cr[0]} (DST) dir {cr[1]:+d} : " + " · ".join(diffs))
        else:
            print(f"  (DST) {cr[0]} dir {cr[1]:+d} : apparié "
                  f"(Δts {ce[0] - cr[0]}s) — conforme")
    for cle in only_rust:
        print(f"  ✗ {cle[0]} dir {cle[1]:+d} : présent chez Rust SEULEMENT")
    for cle in only_ea:
        print(f"  ✗ {cle[0]} dir {cle[1]:+d} : présent chez l'EA SEULEMENT")

    matched = len(cles_communes)
    ecarts += len(only_rust) + len(only_ea)
    print(f"\nMatched {matched} · Divergences {ecarts} "
          f"(rust-only {len(only_rust)}, ea-only {len(only_ea)})")
    if ecarts == 0 and matched > 0:
        print("PARITÉ ✓ — le miroir MQ5 est fidèle au moteur Rust.")
        return 0
    print("Divergences détectées — voir docs/VALIDATION_KDJ_HALFTREND.md §tracage.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
