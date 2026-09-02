<template>
        <div class="flex flex-col gap-3">
        <!-- Concept et moteurs côte à côte (50/50) -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
        <carte titre="Concept">
          SMC trade les mouvements de l'argent institutionnel. Le cycle est toujours le même :
          accumulation (le prix prépare), manipulation (les liquidités apparentes sont chassées),
          distribution (l'impulsion part de la zone institutionnelle). L'analyse se lit en quatre
          étapes, dans l'ordre — chacune conditionne la suivante.
        </carte>

        <carte titre="Les deux moteurs de signaux">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">Moteur Order Blocks</b> — score par zone enrichi
            (fraîcheur + proximité + <b class="text-white">mega-vol</b> : volume ≥ 2× SMA20),
            barème calibré par actif et timeframe. Qualification : score au seuil + force ≥ 4 +
            qualité de zone (retour du prix et proximité).</p>
            <p><b class="text-white">Moteur BSZones</b> — Sweep → Dispersion → OB : zones
            reconstituées après la chasse de liquidité, score propre, seuil dédié (score ≥ 7).</p>
          </div>
        </carte>
        </div>

        <!-- Les 4 étapes de l'analyse SMC, dans l'ordre, en 4 colonnes -->
        <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
          <carte titre="① La structure de marché">
            <svg viewBox="0 0 220 110" class="w-full aspect-[220/110] mb-2">
              <!-- Zigzag haussier puis cassure -->
              <polyline points="15,88 45,45 60,68 95,28 112,52 150,18 168,50 200,72"
                fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <!-- BOS : cassure du sommet précédent (continuation) -->
              <line x1="95" y1="45" x2="212" y2="45" stroke="#34d399" stroke-width="0.9" stroke-dasharray="3 3" />
              <text x="188" y="41" fill="#34d399" font-size="7" font-weight="700">BOS</text>
              <!-- MSS : cassure du dernier creux (retournement) -->
              <line x1="168" y1="52" x2="212" y2="52" stroke="#f87171" stroke-width="0.9" stroke-dasharray="3 3" />
              <text x="196" y="62" fill="#f87171" font-size="7" font-weight="700">MSS</text>
              <!-- Étiquettes pivots -->
              <text x="38" y="38" fill="#ffffff" font-size="7" font-weight="700">H</text>
              <text x="87" y="21" fill="#ffffff" font-size="7" font-weight="700">HH</text>
              <text x="54" y="80" fill="#ffffff" font-size="7" font-weight="700">HL</text>
              <text x="106" y="63" fill="#ffffff" font-size="7" font-weight="700">HL</text>
              <text x="144" y="11" fill="#ffffff" font-size="7" font-weight="700">HH</text>
              <text x="170" y="43" fill="#ffffff" font-size="7" font-weight="700">LH</text>
              <text x="192" y="84" fill="#ffffff" font-size="7" font-weight="700">LL</text>
            </svg>
            <div class="flex flex-col gap-1.5">
              <p><b class="text-white">Lire le sens.</b> Les pivots (swing length calibré par
              actif) dessinent la succession sommets/creux : HH+HL = tendance haussière,
              LH+LL = baissière.</p>
              <p>Le <b class="text-white">BOS</b> (break of structure) confirme la continuation ;
              le <b class="text-white">MSS/CHOCH</b> (cassure avec displacement) annonce le
              retournement.</p>
              <p class="text-white">Structure indécise = aucun signal : le sens cadre tout le
              reste de l'analyse.</p>
            </div>
          </carte>

          <carte titre="② Les liquidités et la chasse">
            <svg viewBox="0 0 220 110" class="w-full aspect-[220/110] mb-2">
              <!-- Niveau de liquidité apparent -->
              <line x1="15" y1="45" x2="205" y2="45" stroke="#fbbf24" stroke-width="1" stroke-dasharray="4 3" />
              <text x="16" y="39" fill="#fbbf24" font-size="7" font-weight="700">PDH — liquidité</text>
              <!-- Prix : monte, MÈCHE AU-DESSUS puis rejet -->
              <polyline points="15,92 45,66 62,74 100,30 118,58 140,48 175,88 205,98"
                fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <!-- Le sweep : mèche qui perce puis referme -->
              <circle cx="100" cy="30" r="3.2" fill="none" stroke="#f87171" stroke-width="1.4" />
              <text x="80" y="20" fill="#f87171" font-size="7" font-weight="700">sweep</text>
              <!-- Rejet = impulsion en contre-attaque -->
              <line x1="140" y1="48" x2="175" y2="88" stroke="#34d399" stroke-width="1.2" />
              <polygon points="175,88 171,80 179,81" fill="#34d399" />
              <text x="166" y="68" fill="#34d399" font-size="7" font-weight="700">rejet</text>
            </svg>
            <div class="flex flex-col gap-1.5">
              <p><b class="text-white">Localiser les cibles.</b> PDH/PDL (plus haut/bas de la
              veille), PWH/PWL (semaine), EQH/EQL (épaules égales), Asian/London High-Low.</p>
              <p>Le <b class="text-white">sweep</b> — mèche qui perce un niveau puis referme —
              révèle la manipulation : c'est le carburant des entrées en contre-attaque.</p>
              <p class="text-white">Tout niveau touché sur barre confirmée (sweep ou cassure) est
              consommé et disparaît du carnet.</p>
            </div>
          </carte>

          <carte titre="③ Les zones d'entrée">
            <svg viewBox="0 0 220 110" class="w-full aspect-[220/110] mb-2">
              <!-- Order Block : dernière bougie opposée avant l'impulsion -->
              <rect x="25" y="62" width="42" height="22" fill="rgba(52,211,153,0.15)" stroke="#34d399" stroke-width="1" />
              <text x="31" y="76" fill="#34d399" font-size="7" font-weight="700">OB</text>
              <!-- Impulsion qui part de la zone -->
              <line x1="67" y1="66" x2="150" y2="22" stroke="#34d399" stroke-width="1.6" />
              <polygon points="150,22 141,24 146,31" fill="#34d399" />
              <text x="86" y="32" fill="#34d399" font-size="7" font-weight="700">impulsion</text>
              <!-- Retest de la zone = entrée -->
              <polyline points="150,22 172,30 160,52 100,62" fill="none" stroke="#fbbf24" stroke-width="1.2" stroke-dasharray="4 3" />
              <circle cx="100" cy="63" r="2.8" fill="#ffffff" />
              <text x="30" y="56" fill="#ffffff" font-size="7" font-weight="700">entrée (retest)</text>
              <!-- Gestion : SL sous la zone, TP au-dessus -->
              <line x1="25" y1="90" x2="205" y2="90" stroke="#f87171" stroke-width="0.9" stroke-dasharray="3 3" />
              <text x="186" y="87" fill="#f87171" font-size="7" font-weight="700">SL</text>
              <line x1="120" y1="45" x2="205" y2="45" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="3 3" />
              <text x="184" y="42" fill="#60a5fa" font-size="7" font-weight="700">TP1</text>
            </svg>
            <div class="flex flex-col gap-1.5">
              <p><b class="text-white">Où l'ordre est posé.</b> Order Blocks (dernière bougie
              opposée avant l'impulsion — lifecycle : vierge → signalé → consommé) et BSZones
              (Sweep → Dispersion → OB).</p>
              <p>Compléments : FVG/imbalance (gap non distribué), IFVG (inversée par BOS),
              Breaker (OB cassé changeant de camp).</p>
              <p class="text-white">Entrée au <b>retest</b> : l'ordre est posé en limite à la
              zone — pas de prix qui revient, pas de trade.</p>
            </div>
          </carte>

          <carte titre="④ Le contexte">
            <svg viewBox="0 0 220 110" class="w-full aspect-[220/110] mb-2">
              <!-- Range : premium (vendre) / discount (acheter) -->
              <rect x="70" y="15" width="80" height="80" fill="none" stroke="rgba(255,255,255,0.35)" stroke-width="1" />
              <rect x="70" y="15" width="80" height="40" fill="rgba(248,113,113,0.10)" />
              <rect x="70" y="55" width="80" height="40" fill="rgba(52,211,153,0.10)" />
              <text x="110" y="26" fill="#f87171" font-size="7" font-weight="700">Premium</text>
              <text x="74" y="91" fill="#34d399" font-size="7" font-weight="700">Discount</text>
              <!-- Équilibre du range -->
              <line x1="70" y1="55" x2="150" y2="55" stroke="#ffffff" stroke-width="0.8" stroke-dasharray="3 3" />
              <text x="154" y="58" fill="#ffffff" font-size="7" font-weight="700">EQ 50 %</text>
              <!-- Bande OTE 62-79 % -->
              <rect x="70" y="64" width="80" height="15" fill="rgba(251,191,36,0.18)" />
              <text x="154" y="74" fill="#fbbf24" font-size="7" font-weight="700">OTE 62-79 %</text>
              <!-- Impulsion puis retrace dans la zone optimale -->
              <polyline points="15,92 85,22 120,71" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <circle cx="120" cy="71" r="2.8" fill="#ffffff" />
              <line x1="123" y1="73" x2="148" y2="89" stroke="#ffffff" stroke-width="0.7" stroke-dasharray="2 2" />
              <text x="152" y="92" fill="#ffffff" font-size="7" font-weight="700">entrée optimale</text>
            </svg>
            <div class="flex flex-col gap-1.5">
              <p><b class="text-white">Décider si ça vaut le coup.</b> Premium/Discount (moitié
              haute/basse du range), OTE (retrace 62-79 % de l'impulsion), Kill Zones de Londres
              (7h-10h UTC) et New York (13h-16h UTC), NDOG/NWOG (gaps journalier/hebdo).</p>
              <p>Confluences multi-timeframe (H1/H4/W1/MN amorcées sur l'historique) et score
              <b class="text-white">/10 par actif</b> : en dessous du seuil, le setup est
              ignoré.</p>
            </div>
          </carte>
        </div>
      </div>
</template>

<script setup lang="ts">
import { Carte as carte } from '@/components/common/carteTitree'
</script>
