#!/usr/bin/env python3
"""Rejeu des transitions du week-end — espion Tendance Kasper vs compositeur.

Action cockpit KAS (19/10) : « script de rejeu des transitions week-end du
journal (croisements stricts, gris, Sunday) contre le compositeur — 1 script,
déclenchable à tout moment ».

PRINCIPE : le journal de l'espion (espion_kasper_YYYY.MM.DD.jsonl, UTF-16LE)
enregistre à chaque poll les EMA du PANNEAU Kasper pour les 7 TF :
  ema = {TF: {e9v, e9c, e21v, e21c}}   (v = bougie vivante, c = clôturée)
Le rejeu reconstruit, à l'horodatage `ts` de CHAQUE entrée, ce que NOTRE
pipeline aurait affiché — miroir fidèle de tendance_handlers.rs :
  1. bougies TF en base avec ts <= T (limite 630, comme le handler) ;
  2. bougie vivante composée des M1 via composer_vivante (ancrage grille
     du TF, k=0 → REMPLACER la barre figée, k≥1 → APPEND) ;
  3. EMA9/EMA21 (graine SMA, récursion identique à indicators::calculer_ema) ;
  4. vivante = dernier index, clôturée = avant-dernier (Modes du handler).
Puis compare états (signe e9−e21) et valeurs au centime.

Le week-end : aucune ligne samedi/dimanche (espion arrêté) — la transition
EST le trou. Le rapport isole le plus grand gap (ven→lun) et vérifie que
notre compositeur reproduit le nouvel état de lundi (« croisement strict à
travers le trou »), les lignes grises (TF absent/null du panneau) et
l'ancrage à la réouverture (« Sunday »).

Auto-calibrage : l'horloge MT5 (server) peut différer de l'epoch unix —
l'offset est mesuré sur les entrées M1 denses (0/±1 h testés, retenu = Δ
médian minimal) puis affiché et appliqué partout.

Usage :
  python3 scripts/rejeu_kas_weekend.py [--asset XAUUSD]
      [--db data/trading.db]
      [--journal-dir "~/.mt5/drive_c/Program Files/MetaTrader 5/MQL5/Files"]
      [--debut "2026-10-16 12:00"] [--fin "2026-10-19 23:59"]   (Paris)
Sortie : rapport texte sur stdout. DB ouverte en LECTURE SEULE.
"""
from __future__ import annotations

import argparse
import bisect
import datetime as dt
import glob
import json
import math
import os
import sqlite3
import statistics
import sys

TFS = [("M1", 60), ("M5", 300), ("M15", 900), ("M30", 1800),
       ("H1", 3600), ("H4", 14400), ("D1", 86400)]
LIMITE_TF = 630          # = max(ema_lente*30, 500) du handler
PARIS = dt.timezone(dt.timedelta(hours=2))  # CEST (le week-end d'octobre)


# ── Journal ───────────────────────────────────────────────────────────────────

def lire_journal(dossier: str, symbole: str) -> list[dict]:
    entrees = []
    for chemin in sorted(glob.glob(os.path.join(dossier, "espion_kasper_2*.jsonl"))):
        try:
            brut = open(chemin, encoding="utf-16").read()
        except (UnicodeError, OSError):
            continue
        for ligne in brut.splitlines():
            ligne = ligne.strip().lstrip("\ufeff")
            if not ligne:
                continue
            try:
                e = json.loads(ligne)
            except json.JSONDecodeError:
                continue  # dernière ligne en cours d'écriture
            if e.get("symbole") == symbole and isinstance(e.get("ema"), dict):
                entrees.append(e)
    entrees.sort(key=lambda e: e["ts"])
    return entrees


# ── Bougies (lecture seule) ───────────────────────────────────────────────────

def charger_bougies(db: sqlite3.Connection, asset: str, tf: str):
    rows = db.execute(
        "SELECT timestamp, open, high, low, close, volume FROM bougies"
        " WHERE asset = ? AND timeframe = ? ORDER BY timestamp",
        (asset, tf),
    ).fetchall()
    bougies = [(int(r[0]), float(r[1]), float(r[2]), float(r[3]), float(r[4]), float(r[5]))
               for r in rows]
    tss = [b[0] for b in bougies]
    return bougies, tss


# ── Miroirs exacts du backend ─────────────────────────────────────────────────

def ema(closes: list[float], periode: int) -> list[float]:
    """Miroir strict d'indicators::calculer_ema (graine SMA, même récursion)."""
    n = len(closes)
    if n < periode or periode == 0:
        return [math.nan] * n
    k = 2.0 / (periode + 1.0)
    ema_v = [math.nan] * n
    seed = sum(closes[:periode]) / periode  # même ordre de sommation
    ema_v[periode - 1] = seed
    for i in range(periode, n):
        ema_v[i] = closes[i] * k + ema_v[i - 1] * (1.0 - k)
    return ema_v


def composer_vivante(tf_t: list[tuple], m1: list[tuple], periode_sec: int, maintenant: int):
    """Miroir strict de tendance_handlers::composer_vivante (ancrage grille)."""
    if not tf_t or not m1:
        return None
    derniere_ts = tf_t[-1][0]
    k = max(0, (maintenant - derniere_ts) // periode_sec)
    debut = derniere_ts + k * periode_sec
    remplacer = k == 0
    fenetre = [c for c in m1 if debut <= c[0] < debut + periode_sec]
    if not fenetre:
        return None
    premiere, derniere_m1 = fenetre[0], fenetre[-1]
    composee = (premiere[0], premiere[1],
                max(c[2] for c in fenetre), min(c[3] for c in fenetre),
                derniere_m1[4], sum(c[5] for c in fenetre))
    return composee, remplacer


def rejeuer(tf_t: list[tuple], tss: list[int], m1: list[tuple], m1_tss: list[int],
            periode_sec: int, tf_minutes: int, T: int):
    """Rejoue tendance_multi_tf (mode bougie_en_cours) à l'instant T.
    Retourne (e9v, e21v, e9c, e21c) ou None si pas de vivante.

    ANTI-LOOKAHEAD (spécifique au rejeu) : la base lue AUJOURD'HUI contient
    la close FINALE des barres formantes d'alors — les garder fausserait
    l'EMA d'un biais croissant avec la période (mesuré : M1 +0,2 $ … H1
    +2,4 $). On ne garde donc que les barres CLÔTURÉES à T (ts+P ≤ T) et
    la vivante est composée des M1 ≤ T — miroir exact de ce que le
    handler voit en direct (à l'exécution, la barre formante en base
    porte bien la close « telle quelle »)."""
    fin = bisect.bisect_right(tss, T - periode_sec)   # dernière clôturée à T
    toutes = tf_t[max(0, fin - LIMITE_TF):fin]
    if len(toutes) < 22:
        return None
    profondeur = min(tf_minutes + 120, 10_800)          # comme le handler
    fmin = bisect.bisect_right(m1_tss, T)
    m1_t = m1[max(0, fmin - profondeur):fmin]
    comp = composer_vivante(toutes, m1_t, periode_sec, T)
    if comp is None:
        return None
    vivante, remplacer = comp
    tableau = toutes[:-1] + [vivante] if remplacer else toutes + [vivante]
    closes = [c[4] for c in tableau]
    e9, e21 = ema(closes, 9), ema(closes, 21)
    if len(closes) < 2:
        return None
    return e9[-1], e21[-1], e9[-2], e21[-2]              # vivante / clôturée


def etat(rapide: float, lente: float) -> str | None:
    if math.isnan(rapide) or math.isnan(lente):
        return None
    if rapide > lente:
        return "H"
    if rapide < lente:
        return "B"
    return None


# ── Rapport ───────────────────────────────────────────────────────────────────

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--asset", default="XAUUSD")
    ap.add_argument("--db", default="data/trading.db")
    ap.add_argument("--journal-dir",
                    default="~/.mt5/drive_c/Program Files/MetaTrader 5/MQL5/Files")
    ap.add_argument("--debut", default=None, help="YYYY-MM-DD HH:MM (Paris)")
    ap.add_argument("--fin", default=None, help="YYYY-MM-DD HH:MM (Paris)")
    args = ap.parse_args()

    dossier = os.path.expanduser(args.journal_dir)
    entrees = lire_journal(dossier, args.asset)
    if not entrees:
        print(f"Aucune entrée journal pour {args.asset} dans {dossier}")
        return 1

    def serv(e: dict) -> str:
        """Heure serveur de l'entrée (horloge MT5 du panneau)."""
        h = str(e.get("heure") or e["ts"])
        return f"{h[8:10]}/{h[5:7]} {h[11:16]}"

    if args.debut:
        d = dt.datetime.strptime(args.debut, "%Y-%m-%d %H:%M").replace(tzinfo=dt.timezone.utc)
        entrees = [e for e in entrees if e["ts"] >= int(d.timestamp())]
    if args.fin:
        f = dt.datetime.strptime(args.fin, "%Y-%m-%d %H:%M").replace(tzinfo=dt.timezone.utc)
        entrees = [e for e in entrees if e["ts"] <= int(f.timestamp())]
    if not entrees:
        print("Fenêtre vide.")
        return 1

    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    donnees = {tf: charger_bougies(db, args.asset, tf) for tf, _ in TFS}
    db.close()
    tf_bougies = {tf: b for tf, (b, _) in donnees.items()}
    tf_tss = {tf: t for tf, (_, t) in donnees.items()}
    m1, m1_tss = tf_bougies["M1"], tf_tss["M1"]
    manquants = [tf for tf, b in tf_bougies.items() if not b]
    if manquants:
        print(f"⚠ bougies absentes en base : {manquants}")

    print(f"Journal : {len(entrees)} entrées {args.asset} "
          f"({serv(entrees[0])} → {serv(entrees[-1])} Paris)")

    # ── Auto-calibrage de l'horloge serveur (M1 dense) ──
    # Mesuré 19/10 : les bougies DB vivent en epochs UTC réels (dernière M1
    # à ~100 s de l'heure système) ; le journal KAS est horodaté par
    # l'horloge SERVEUR MT5 = heure Paris rendue comme epoch (+2 h en CEST).
    def delta_median(off: int) -> float:
        ds = []
        for e in entrees[::25]:
            v = e["ema"].get("M1")
            if not v or "e9v" not in v:
                continue
            r = rejeuer(tf_bougies["M1"], tf_tss["M1"], m1, m1_tss, 60, 1, e["ts"] + off)
            if r and not math.isnan(r[0]):
                ds.append(abs(r[0] - v["e9v"]))
        return statistics.median(ds) if ds else math.inf

    essais = {"0 (journal=UTC)": 0, "−1h": -3600, "−2h (journal=heure Paris)": -7200,
              "−3h": -10_800}
    scores = {nom: delta_median(off) for nom, off in essais.items()}
    retenu = min(scores, key=scores.get)
    offset = essais[retenu]
    print("Calibrage horloge : " + ", ".join(f"{n}={s:.4f}" for n, s in scores.items())
          + f" → retenu {retenu} (Δ médian {scores[retenu]:.4f})")

    # ── Rejeu complet ──
    tot = match = 0
    gris = 0
    ecarts_v: list[float] = []
    ecarts_c: list[float] = []
    divergences = []
    marges_div = []      # |e9v−e21v| panneau au moment d'une divergence
    marges_ok = []       # idem quand les états concordent
    for e in entrees:
        T = e["ts"] + offset
        for tf, sec in TFS:
            v = e["ema"].get(tf)
            if not v or v.get("e9v") in (None, 0) or v.get("e21v") in (None, 0):
                gris += 1
                continue
            r = rejeuer(tf_bougies[tf], tf_tss[tf], m1, m1_tss, sec, sec // 60, T)
            tot += 1
            if r is None:
                divergences.append((e["ts"], tf, "rejeu=None",
                                    f"panneau e9v={v['e9v']:.2f}"))
                continue
            e9v, e21v, e9c, e21c = r
            dv, dc = abs(e9v - v["e9v"]), abs(e9c - v["e9c"])
            ecarts_v.append(dv)
            if not math.isnan(dc):
                ecarts_c.append(dc)
            sv_p, sv_r = etat(v["e9v"], v["e21v"]), etat(e9v, e21v)
            marge = abs(v["e9v"] - v["e21v"])
            if sv_p == sv_r:
                match += 1
                marges_ok.append(marge)
            else:
                marges_div.append(marge)
                divergences.append((e["ts"], tf, f"état {sv_p}→{sv_r}",
                                    f"panneau e9v={v['e9v']:.2f} e21v={v['e21v']:.2f}"
                                    f" · rejeu e9v={e9v:.2f} e21v={e21v:.2f}"))

    print(f"\n── REJEU GLOBAL ({len(entrees)} entrées × 7 TF) ──")
    print(f"états vivante conformes : {match}/{tot}"
          f" ({100 * match / max(tot, 1):.2f} %)")
    if ecarts_v:
        print(f"|Δ e9v| médian={statistics.median(ecarts_v):.4f}"
              f" max={max(ecarts_v):.4f}  (centimes de $)")
    if ecarts_c:
        print(f"|Δ e9c| médian={statistics.median(ecarts_c):.4f}"
              f" max={max(ecarts_c):.4f}")
    print(f"lignes grises (TF absent du panneau) : {gris}")
    if marges_div and marges_ok:
        print(f"marge panneau |e9v−e21v| : médiane divergences "
              f"{statistics.median(marges_div):.2f} $ vs concordes "
              f"{statistics.median(marges_ok):.2f} $ — les écarts d'état se "
              f"concentrent au voisinage des croisements (EMA panneau "
              f"vraisemblablement récursive au tick, non sur closes).")
    if divergences:
        print(f"divergences : {len(divergences)}")
        for ts, tf, cause, detaille in divergences[:12]:
            h = dt.datetime.fromtimestamp(ts, dt.timezone.utc).strftime("%d/%m %Hh%M")
            print(f"  {h} {tf}: {cause} — {detaille}")
        if len(divergences) > 12:
            print(f"  … +{len(divergences) - 12} autres")

    # ── Transitions du week-end : le plus grand gap du journal ──
    gaps = [(entrees[i + 1]["ts"] - entrees[i]["ts"], i)
            for i in range(len(entrees) - 1)]
    if not gaps:
        return 0
    duree, i = max(gaps)
    avant, apres = entrees[i], entrees[i + 1]
    print(f"\n── TRANSITION WEEK-END (plus grand gap : {duree / 3600:.1f} h) ──")
    print(f"  {serv(avant)} → {serv(apres)} Paris")
    print(f"  {'TF':4} {'ven (panneau)':>14} {'lun (panneau)':>14}"
          f" {'lun (rejeu)':>12}  croisement")
    croises = []
    for tf, _ in TFS:
        va, vl = avant["ema"].get(tf), apres["ema"].get(tf)
        if not va or not vl:
            print(f"  {tf:4} {'gris/absent':>14}")
            continue
        sa = etat(va.get("e9v", math.nan), va.get("e21v", math.nan))
        sl = etat(vl.get("e9v", math.nan), vl.get("e21v", math.nan))
        sec = dict(TFS)[tf]
        r = rejeuer(tf_bougies[tf], tf_tss[tf], m1, m1_tss, sec, sec // 60, apres["ts"] + offset)
        sr = etat(r[0], r[1]) if r else None
        marque = "" if sa == sl else "  ← CROISEMENT"
        if sa != sl:
            croises.append(tf)
        print(f"  {tf:4} {str(sa):>14} {str(sl):>14} {str(sr):>12}{marque}")
    if croises:
        print(f"\nCroisements stricts à travers le trou : {', '.join(croises)}")
        print("→ le rejeu lundi doit afficher le MÊME état que le panneau ;"
              " toute divergence ci-dessus = compositeur en cause.")

    # ── Réouverture (« Sunday ») : premières entrées après le gap ──
    print(f"\n── RÉOUVERTURE (ancrage grille à la reprise) ──")
    for e in entrees[i + 1:i + 6]:
        T = e["ts"] + offset
        ligne = []
        for tf, sec in TFS:
            r = rejeuer(tf_bougies[tf], tf_tss[tf], m1, m1_tss, sec, sec // 60, T)
            v = e["ema"].get(tf) or {}
            ok = "·" if (r is None and not v.get("e9v")) or (
                r and v.get("e9v") and etat(r[0], r[1]) == etat(v["e9v"], v["e21v"])) else "✗"
            ligne.append(f"{tf}:{ok}")
        print(f"  {serv(e)} " + " ".join(ligne))
    return 0


if __name__ == "__main__":
    sys.exit(main())
