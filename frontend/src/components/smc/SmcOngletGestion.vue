<template>
  <div class="flex flex-col gap-3">
    <carte titre="La construction des niveaux">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
        <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
          <!-- Échelle des niveaux, proportionnelle en R -->
          <line x1="68" y1="10" x2="290" y2="10" stroke="#34d399" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="298" y="13" fill="#34d399" font-size="7.5" font-weight="700">TP3 · liquidité lointaine / R fixe</text>
          <line x1="68" y1="32" x2="290" y2="32" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="298" y="35" fill="#60a5fa" font-size="7.5" font-weight="700">TP2 · +2R (défaut)</text>
          <line x1="68" y1="63" x2="290" y2="63" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="298" y="66" fill="#60a5fa" font-size="7.5" font-weight="700">TP1 · +0,6R (défaut)</text>
          <line x1="68" y1="76" x2="290" y2="76" stroke="#ffffff" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="298" y="79" fill="#ffffff" font-size="7.5" font-weight="700">entrée · 0R</text>
          <line x1="68" y1="98" x2="290" y2="98" stroke="#f87171" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="298" y="101" fill="#f87171" font-size="7.5" font-weight="700">SL · −1R</text>
          <!-- Zone d'origine -->
          <rect x="25" y="68" width="40" height="16" fill="rgba(52,211,153,0.15)" stroke="#34d399" stroke-width="1" />
          <text x="50" y="74" fill="#34d399" font-size="7.5" font-weight="700">OB</text>
          <!-- Trajectoire : retest, TP1 (stop → BE), repli, TP2, TP3 -->
          <polyline points="25,76 45,88 65,82 95,63 115,68 150,76 185,48 220,32 255,20 285,10" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
          <circle cx="95" cy="63" r="2.6" fill="#ffffff" />
          <circle cx="220" cy="32" r="2.6" fill="#ffffff" />
          <circle cx="285" cy="10" r="2.6" fill="#ffffff" />
          <text x="120" y="52" fill="#e5e7eb" font-size="7" font-weight="700">stop → BE</text>
        </svg>
        <div class="flex flex-col gap-2">
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">Stop Loss</div>
            <p>Bord opposé de la zone ± offset ATR (réduit de 25 %), distance clampée entre
            slMin et slMax : multiplicateurs ×ATR calibrés par actif — BTC 0,8-2,5, or
            0,5-1,5, NAS/DAX 0,5-1,5, argent 0,6-1,8.</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">TP1 / TP2</div>
            <p>TP1 <b class="text-white">réglable</b> (défaut 0,6, borné 0,2-1,5), TP2
            <b class="text-white">réglable</b> (défaut 2, borné 1,0-4,0) et TP3
            <b class="text-white">réglable</b> : <b class="text-white">liquidité lointaine</b>
            (la plus lointaine des EQH/PDH/PWH, repli R fixe si absente ou sous TP2) ou
            <b class="text-white">R fixe</b> (3-10R) — cascade TP1 &lt; TP2 &lt; R fixe.
            Page Stratégie SMC › ⚙️ Paramètres, effet au prochain armement. R = distance
            entrée-stop après clamp.</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">TP3</div>
            <p>La liquidité la plus proche au-delà de l'entrée (EQH, PDH, PWH, Asian High pour
            un achat) ; repli sur +3R si aucune cible ou monotonie brisée.</p>
          </div>
        </div>
      </div>
    </carte>

    <carte titre="Le cycle de vie avec ventes partielles">
      <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
        <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
        <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Remplissage</text>
        <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">3 lots partiels posés</text>
        <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
        <text x="210" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP1 · 1re vente</text>
        <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">stop → BE</text>
        <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
        <text x="350" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">TP2 · 2e vente</text>
        <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">solde sous BE</text>
        <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
        <text x="490" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP3 · solde</text>
        <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cible ou trailing</text>
        <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
        <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
        <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
        <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
        <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
        <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
      </svg>
      <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-3">
        <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
          <div class="font-semibold mb-1">① Remplissage</div>
          <p>Le trade entre au retest : le lot est coupé en trois lots partiels, aux fractions
          réglées dans Paramètres › stratégies › SMC.</p>
        </div>
        <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
          <div class="font-semibold mb-1">② TP1 touché</div>
          <p>Première vente partielle (ex. 50 %) à +0,6R. Le stop remonte à l'entrée :
          le solde est break-even garanti.</p>
        </div>
        <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
          <div class="font-semibold mb-1">③ TP2 touché</div>
          <p>Deuxième vente (ex. 30 %) à +2R. Le solde reste sous protection
          break-even.</p>
        </div>
        <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
          <div class="font-semibold mb-1">④ Retour sous TP1</div>
          <p>Si le prix redescend, le solde sort à break-even — les parts déjà vendues
          restent encaissées.</p>
        </div>
        <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
          <div class="font-semibold mb-1">⑤ TP3 touché</div>
          <p>Le solde (ex. 20 %) est vendu sur la liquidité visée : clôture complète
          du trade.</p>
        </div>
      </div>
    </carte>

    <carte titre="Les ventes partielles — le R pondéré">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
        <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
          <!-- Échelle + part du lot vendue à chaque palier -->
          <line x1="68" y1="10" x2="250" y2="10" stroke="#34d399" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="258" y="13" fill="#34d399" font-size="7.5" font-weight="700">TP3 · cible</text>
          <text x="322" y="13" fill="#34d399" font-size="6.5" font-weight="700">solde 20 %</text>
          <line x1="68" y1="32" x2="250" y2="32" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="258" y="35" fill="#60a5fa" font-size="7.5" font-weight="700">TP2 · +2R</text>
          <text x="322" y="35" fill="#60a5fa" font-size="6.5" font-weight="700">vente 30 %</text>
          <line x1="68" y1="63" x2="250" y2="63" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="258" y="66" fill="#60a5fa" font-size="7.5" font-weight="700">TP1 · +0,6R</text>
          <text x="322" y="66" fill="#60a5fa" font-size="6.5" font-weight="700">vente 50 %</text>
          <line x1="68" y1="76" x2="250" y2="76" stroke="#ffffff" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="258" y="79" fill="#ffffff" font-size="7.5" font-weight="700">entrée · 0R</text>
          <text x="322" y="79" fill="#e5e7eb" font-size="6.5" font-weight="700">3 lots posés</text>
          <line x1="68" y1="98" x2="250" y2="98" stroke="#f87171" stroke-width="0.9" stroke-dasharray="4 3" />
          <text x="258" y="101" fill="#f87171" font-size="7.5" font-weight="700">SL · −1R</text>
          <text x="322" y="101" fill="#f87171" font-size="6.5" font-weight="700">tout le lot</text>
          <!-- Trajectoire : trois sorties partielles -->
          <polyline points="25,76 45,88 65,82 95,63 115,68 150,76 185,48 220,32 240,20 250,11" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
          <!-- Trailing stop après TP2 (réglable, inactif par défaut) : le stop
               suit l'extrême post-TP2 à k×R — sortie avant la cible possible -->
          <polyline points="220,32 240,20 250,11" fill="none" stroke="#22d3ee" stroke-width="1" stroke-dasharray="3 2" />
          <text x="380" y="13" fill="#22d3ee" font-size="6" font-weight="700">trailing k×R</text>
          <line x1="255" y1="10" x2="376" y2="10" stroke="#22d3ee" stroke-width="0.8" stroke-dasharray="2 2" />
          <circle cx="95" cy="63" r="2.6" fill="#ffffff" />
          <circle cx="220" cy="32" r="2.6" fill="#ffffff" />
          <circle cx="250" cy="11" r="2.6" fill="#ffffff" />
          <text x="120" y="52" fill="#e5e7eb" font-size="7" font-weight="700">stop → BE</text>
        </svg>
        <div class="flex flex-col gap-2">
          <!-- Répartition du lot (défaut 50/30/20) -->
          <div class="flex h-4 rounded overflow-hidden text-[8px] font-bold text-white">
            <div class="flex items-center justify-center" style="width:50%;background:rgba(96,165,250,0.55)">50 % · TP1</div>
            <div class="flex items-center justify-center" style="width:30%;background:rgba(96,165,250,0.32)">30 % · TP2</div>
            <div class="flex items-center justify-center" style="width:20%;background:rgba(52,211,153,0.45)">20 % · TP3</div>
          </div>
          <table class="w-full text-xs">
            <thead>
              <tr class="text-[10px] uppercase tracking-wide text-white border-b border-white/10">
                <th class="text-left py-1 pr-2">Verdict</th>
                <th class="text-left py-1 pr-2">Vendu à TP1</th>
                <th class="text-left py-1 pr-2">Vendu à TP2</th>
                <th class="text-left py-1 pr-2">Solde</th>
                <th class="text-right py-1">R pondéré</th>
              </tr>
            </thead>
            <tbody class="text-white">
              <tr class="border-b border-white/5">
                <td class="py-1 pr-2 font-semibold text-emerald-400">TP3</td>
                <td class="py-1 pr-2">50 % à +0,6R</td><td class="py-1 pr-2">30 % à +2R</td><td class="py-1 pr-2">20 % à la cible</td>
                <td class="py-1 text-right font-mono font-bold text-emerald-400">+1,50R</td>
              </tr>
              <tr class="border-b border-white/5">
                <td class="py-1 pr-2 font-semibold">TP2 + BE</td>
                <td class="py-1 pr-2">50 % à +0,6R</td><td class="py-1 pr-2">30 % à +2R</td><td class="py-1 pr-2">20 % à 0R</td>
                <td class="py-1 text-right font-mono font-bold">+0,90R</td>
              </tr>
              <tr class="border-b border-white/5">
                <td class="py-1 pr-2 font-semibold">TP1 + BE</td>
                <td class="py-1 pr-2">50 % à +0,6R</td><td class="py-1 pr-2">—</td><td class="py-1 pr-2">50 % à 0R</td>
                <td class="py-1 text-right font-mono font-bold">+0,30R</td>
              </tr>
              <tr class="border-b border-white/5">
                <td class="py-1 pr-2 font-semibold text-red-400">SL</td>
                <td class="py-1 pr-2">—</td><td class="py-1 pr-2">—</td><td class="py-1 pr-2">100 % à −1R</td>
                <td class="py-1 text-right font-mono font-bold text-red-400">−1R</td>
              </tr>
              <tr class="border-b border-white/5">
                <td class="py-1 pr-2 font-semibold">BE forcé</td>
                <td class="py-1 pr-2">—</td><td class="py-1 pr-2">—</td><td class="py-1 pr-2">100 % à 0R</td>
                <td class="py-1 text-right font-mono font-bold">0</td>
              </tr>
              <tr>
                <td class="py-1 pr-2 font-semibold">Expire</td>
                <td class="py-1 pr-2" colspan="3">Selon les paliers atteints — solde au prix de sortie</td>
                <td class="py-1 text-right font-mono font-bold">R réel</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <p class="mt-3">Tableau aux défauts TP1 = 0,6, TP2 = 2 et repli TP3 = 3R. Fractions réglables dans
      <b class="text-white">Paramètres › stratégies › SMC</b> (Σ = 100 %, défaut 50/30/20) :
      la simulation se recalcule depuis l'historique à chaque changement. Le R pondéré
      alimente <b class="text-white">uniquement la simulation de capital</b> (badge $ et
      courbe bleue du dashboard) — la courbe R de référence et les Σ R du moteur restent
      l'étalon, intouchés.</p>
    </carte>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
      <carte titre="Les sorties anticipées">
        <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
          <!-- BOS opposé : le stop saute à l'entrée -->
          <line x1="20" y1="64" x2="300" y2="64" stroke="#ffffff" stroke-width="0.8" stroke-dasharray="4 3" />
          <text x="86" y="72" fill="#e5e7eb" font-size="7" font-weight="700">entrée</text>
          <line x1="248" y1="30" x2="248" y2="78" stroke="#f87171" stroke-width="1" stroke-dasharray="3 3" />
          <text x="180" y="28" fill="#f87171" font-size="7" font-weight="700">BOS opposé</text>
          <polyline points="20,64 70,48 110,54 150,38 190,56 230,50 265,60 320,64" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
          <polyline points="320,64 360,82" fill="none" stroke="rgba(255,255,255,0.3)" stroke-width="1.2" stroke-dasharray="3 3" />
          <line x1="252" y1="64" x2="340" y2="64" stroke="#60a5fa" stroke-width="2" />
          <text x="262" y="86" fill="#60a5fa" font-size="7" font-weight="700">stop → BE</text>
          <circle cx="320" cy="64" r="3" fill="#ffffff" />
          <text x="334" y="58" fill="#ffffff" font-size="7" font-weight="700">sortie à 0</text>
        </svg>
        <div class="flex flex-col gap-2">
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">SL touché</div>
            <p>Le stop est atteint avant TP1 : le lot entier sort à −1R, sans exception.</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">BE forcé</div>
            <p>Un BOS opposé pendant le trade ramène le stop à l'entrée, même sans TP1 :
            le lot entier sort à 0 — aucune partielle n'a été encaissée.</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
            <div class="font-semibold mb-1">Annulation</div>
            <p>Ordre en attente + BOS opposé : l'ordre est retiré, le trade n'existera jamais.</p>
          </div>
        </div>
      </carte>
      <carte titre="L'expiration">
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 mb-3">
          <valeur etiquette="Intraday" valeur="4 h" />
          <valeur etiquette="H1" valeur="8 h" />
          <valeur etiquette="H4" valeur="32 h" />
          <valeur etiquette="D1" valeur="4 j" />
        </div>
        <p>Au-delà de ce délai, le trade est clos au marché (verdict « Expire »). Après TP2,
        TP3 doit être atteint dans le délai restant. Un trade qui dort ne mérite pas de
        capitale.</p>
      </carte>
    </div>

    <carte titre="Les verdicts">
      <div class="flex flex-wrap gap-2 mb-2">
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TP3</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TP2</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-blue-400 border-blue-400/40 bg-blue-400/10">TP1</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-white border-white/40 bg-white/10">BE</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">SL</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-cyan-400 border-cyan-400/40 bg-cyan-400/10">TS</span>
        <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">Expire</span>
      </div>
      <p>Chaque trade clôturé reçoit son verdict, écrit en base avec le prix de sortie et le R
      réel — la base reste l'étalon de ce qui s'est réellement passé. Les métriques du
      dashboard (Σ R, WR, capital $ — période « depuis l'armement universel ») sont
      re-dérivées du <b class="text-white">TP1 réglé</b> par le re-jeu paramétrique : elles se
      recalculent automatiquement au changement de réglage. Le capital comptera le R
      <b class="text-white">pondéré</b> quand les ventes partielles seront livrées.
      <b class="text-white">TS</b> = sortie sur trailing stop après TP2 (stop suivi à
      k×R de l'extrême post-TP2 — réglable, inactif par défaut) : palier TP2, R réel
      de la sortie. Le capital simulé compose le R <b class="text-white">pondéré</b>
      après ventes partielles ; les Σ R restent ceux du moteur. Aucun message de
      clôture sur Telegram : seule l'imminence parle.</p>
    </carte>
  </div>
</template>

<script setup lang="ts">
import { Carte as carte, Valeur as valeur } from '@/components/common/carteTitree'
</script>
