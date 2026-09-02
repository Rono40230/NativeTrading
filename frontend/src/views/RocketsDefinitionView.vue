<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">📐 Les caractéristiques de la stratégie Rockets</h1>
      <span v-if="reglages" class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border" :class="badgeClasse">{{ reglages.etat }}</span>
    </div>

    <div class="flex gap-1 border-b border-white/10 shrink-0 overflow-x-auto">
      <button v-for="t in onglets" :key="t" class="px-4 py-2 text-sm font-medium whitespace-nowrap transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-blue-400' : 'text-white border-transparent hover:text-white/70'"
        @click="onglet = t">{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ ONGLET DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
            <!-- Pivot -->
            <line x1="20" y1="45" x2="420" y2="45" stroke="#fbbf24" stroke-width="1" stroke-dasharray="5 3" />
            <text x="24" y="40" fill="#fbbf24" font-size="7" font-weight="700">pivot</text>
            <!-- Contractions décroissantes : la base se resserre -->
            <polyline points="20,82 45,58 62,74 85,55 100,70 120,58 132,68 150,60 160,66 172,62 180,65" fill="none" stroke="#e5e7eb" stroke-width="1.2" stroke-linejoin="round" />
            <text x="60" y="96" fill="#e5e7eb" font-size="7" font-weight="700">contractions décroissantes</text>
            <!-- Cassure : marubozu à fort volume -->
            <rect x="190" y="22" width="11" height="43" fill="#34d399" />
            <line x1="195.5" y1="22" x2="195.5" y2="12" stroke="#34d399" stroke-width="1.6" />
            <polygon points="195.5,8 192,15 199,15" fill="#34d399" />
            <circle cx="195" cy="45" r="2.8" fill="#ffffff" />
            <text x="215" y="20" fill="#34d399" font-size="7" font-weight="700">marubozu · volume ≥ 150 % MM50</text>
            <!-- La fusée -->
            <polyline points="201,40 230,32 260,26 272,22" fill="none" stroke="#34d399" stroke-width="1.4" stroke-linejoin="round" />
          </svg>
          Rockets capte les mouvements de volatilité violents et brefs nés d'une compression :
          après une base où la volatilité et les volumes s'assèchent, la cassure du pivot sur
          un chandelier marubozu à fort volume déclenche la fusée. La stratégie chasse la
          sortie de base d'actifs déjà en tendance forte, qui surperforment le marché.
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le classement — 10 points, 4 piliers">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
                <div class="font-semibold mb-1">Fondamental · 3 pts</div>
                <p>Sentiment de marché (BTC haussier + secteur en tendance), contexte (sortie
                de large base, 1ère base), news catalyseur.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
                <div class="font-semibold mb-1">Technique · 3 pts</div>
                <p>Tendance (prix &gt; MM50 &gt; MM200 empilées, à moins de 25 % du plus haut
                52 sem.), volatilité (compression Bollinger puis expansion), intérêt (volumes
                asséchés puis explosés).</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
                <div class="font-semibold mb-1">Chartisme · 2 pts</div>
                <p>Figure de continuation (VCP / tasse avec anse, contractions décroissantes,
                micro-base serrée), pas de gros gaps.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
                <div class="font-semibold mb-1">Chandeliers · 2 pts</div>
                <p>Cassure marubozu (corps ≥ 80 %, +3-5 % au-delà du pivot, volume ≥ 150 %
                MM50), pas de mèches excessives.</p>
              </div>
            </div>
          </carte>
          <carte titre="La classification et le véto">
            <div class="flex flex-col gap-2 mb-3">
              <div class="flex items-center justify-between rounded-lg border border-emerald-400/40 bg-emerald-400/10 px-3.5 py-2">
                <span class="text-sm font-bold text-emerald-400">9-10 · ROCKET ALPHA</span>
                <span class="text-xs text-white">trading neutre / offensif</span>
              </div>
              <div class="flex items-center justify-between rounded-lg border border-blue-400/40 bg-blue-400/10 px-3.5 py-2">
                <span class="text-sm font-bold text-blue-400">7-8 · ROCKET</span>
                <span class="text-xs text-white">trading neutre</span>
              </div>
              <div class="flex items-center justify-between rounded-lg border border-red-400/40 bg-red-400/10 px-3.5 py-2">
                <span class="text-sm font-bold text-red-400">&lt; 7 · ÉLIMINÉ</span>
                <span class="text-xs text-white">hors périmètre</span>
              </div>
            </div>
            <div class="rounded-lg border border-red-400/30 bg-red-400/5 px-3.5 py-3">
              <div class="font-semibold text-red-400 mb-1">Véto éliminatoire</div>
              <p>Un déverrouillage de tokens majeur (≥ 1-2 % de la supply flottante) dans les
              30 prochains jours élimine le candidat, quel que soit son classement — l'étude
              de 16 000 unlocks montre 90 % de pression vendeuse.</p>
            </div>
          </carte>
        </div>

        <carte titre="Le périmètre — deux univers, même classement /10">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1"><span class="text-amber-300">Crypto</span></div>
              <p>Scan quotidien du top 300 Binance en volume (blacklist des paires figées),
              détection sur bougies quotidiennes.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1"><span class="text-blue-300">Actions US</span></div>
              <p>Extension du 01/09, en Observation silencieuse (journalisation seule, aucun
              signal) : répertoire officiel NASDAQ Trader (~5 667 actions communes), prix D1
              Tiingo en volume réel, marché de référence QQQ (même source). Pré-screen trend
              template Minervini (8 conditions) puis le même classement /10. Dépêches Yahoo
              Finance par ticker pour le point news, avertissement 📊 avant résultats (badge,
              pas de veto).</p>
            </div>
          </div>
        </carte>
      </div>

      <!-- ═══ ONGLET LEXIQUE ═══ -->
      <div v-if="onglet === 'Lexique'" class="flex flex-col gap-3">
        <LexiquePanel source="rockets" />
      </div>

      <!-- ═══ ONGLET DÉCISION D'ENTRÉE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="Le parcours d'un signal">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Scan D1</text>
            <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">clôture · 00h40 UTC</text>
            <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="210" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Candidats ≥ 5</text>
            <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">journalisés · Scanner</text>
            <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(251,191,36,0.08)" stroke="#fbbf24" stroke-width="1" />
            <text x="350" y="37.5" text-anchor="middle" fill="#fbbf24" font-size="8" font-weight="700">Cassure du pivot</text>
            <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">marubozu · volume 150 %</text>
            <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="490" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Signal</text>
            <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">si classement ≥ 7</text>
            <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Le scan</div>
              <p>Le scanner quotidien classe l'univers après la clôture D1 — crypto 00h40 UTC,
              actions US 22h30 UTC.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Les candidats</div>
              <p>Les candidats ≥ 5 points sont journalisés et suivis — page Scanner.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ La cassure</div>
              <p>Le pivot casse sur la bougie D1 : décisive (+3 % minimum), marubozu, volume
              ≥ 150 % de la MM50.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ L'ordre</div>
              <p>Stop-limit : achat au-delà du pivot, plafond à la limite (+3 %) pour contenir
              le slippage d'une cassure violente.</p>
            </div>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="La force relative">
            Sans surperformance, pas de point Tendance : l'actif doit battre BTC sur 4 semaines
            (proxy actuel du « secteur en tendance » — le vrai découpage par écosystème viendra
            avec l'IA, étape 6). C'est le critère commun d'O'Neil (RS ≥ 80) et Minervini.
          </carte>
          <carte titre="Ce qui manque encore (honnête)">
            Le point « News » (1/10) et le véto unlocks demandent des sources externes et de la
            lecture — réservés à l'enrichissement IA (étape 6). Le classement actuel est donc
            noté sur 9 chiffrables ; le seuil d'élimination reste 7.
          </carte>
        </div>
      </div>

      <!-- ═══ ONGLET GESTION ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="Le cycle de vie (logique du Journal de Trading)">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Entrée</text>
            <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">stop-limit au pivot</text>
            <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="210" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">R1 atteint</text>
            <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">vendre 50 % · trailing</text>
            <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(251,191,36,0.08)" stroke="#fbbf24" stroke-width="1" />
            <text x="350" y="37.5" text-anchor="middle" fill="#fbbf24" font-size="8" font-weight="700">Trailing</text>
            <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">à la clôture D1</text>
            <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(167,139,250,0.08)" stroke="#a78bfa" stroke-width="1" />
            <text x="490" y="37.5" text-anchor="middle" fill="#a78bfa" font-size="8" font-weight="700">Sortie</text>
            <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">solde vendu</text>
            <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Entrée</div>
              <p>Stop-limit au pivot ; invalidation sous la dernière contraction (−1R).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② R1 atteint</div>
              <p><b class="text-white">Vendre 50 %</b> de la position (fixe) et poser le trailing
              stop à X % du prix (défaut 5 %, réglable).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Trailing</div>
              <p>Suit le prix à la clôture de chaque bougie, jamais vers l'arrière.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ Sortie</div>
              <p>Le prix touche le trailing : le solde est vendu. P&amp;L = 50 % à R1 + solde à
              la sortie.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold text-red-400 mb-1">⑤ Sortie sèche</div>
              <p>Invalidation touchée avant R1 : −1R.</p>
            </div>
          </div>
        </carte>

        <carte titre="Les verdicts">
          <div class="flex flex-wrap gap-2 mb-2">
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">SL · −1R</span>
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TS · 0,5R + solde</span>
          </div>
          <p>Chaque trade clôturé reçoit son verdict et son R réel — TS est un R mixte : 0,5 R
          sécurisé à R1 + 0,5 × R de sortie. Ils alimentent la courbe de trades du bloc. En
          Observation : journalisé, silencieux sur Telegram.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Les profils de risque (du Journal de Trading)">
          <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
            <valeur etiquette="Peu risqué · par rocket" valeur="0,5 %" />
            <valeur etiquette="Neutre · par rocket" valeur="1 %" />
            <valeur etiquette="Risqué · par rocket" valeur="2 %" />
          </div>
          <p class="mt-3">Le profil est un <b class="text-white">choix du propriétaire</b> dans
          les paramètres (comme au journal), jamais déduit du classement — décision actée.
          ETF : 2 % / 3 % / 4 % selon le profil.</p>
        </carte>

        <carte titre="La formule de la quantité">
          <div class="flex flex-wrap items-center justify-center gap-x-3 gap-y-2 font-mono text-xl py-3">
            <span class="text-violet-400 font-bold">quantité</span>
            <span class="text-white">=</span>
            <span class="text-white">(</span>
            <span class="font-bold">capital</span>
            <span class="text-white">×</span>
            <span class="text-blue-400 font-bold">profil</span>
            <span class="text-white">) ÷</span>
            <span class="text-amber-400 font-bold">|entrée − stop|</span>
          </div>
          <p class="text-center">Plafonnée à 5 % du capital en montant par position.</p>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par rocket" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>
      </div>

      <!-- ═══ ONGLET SCANNER (spécifique Rockets) ═══ -->
      <div v-if="onglet === 'Scanner'" class="flex flex-col gap-3">
        <carte titre="Le scanner">
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
            <valeur etiquette="Scan crypto" valeur="00h40 UTC" />
            <valeur etiquette="Scan actions US" valeur="22h30 UTC" />
          </div>
          <p>Crypto : chaque jour après la clôture D1 (00h40 UTC), le top 300 Binance en volume
          est classé. Actions US : chaque jour à 22h30 UTC (après la clôture de Wall Street),
          pré-screen trend template puis classement des passants.</p>
          <p>Les candidats ≥ 5 points des deux univers vivent ici — en attente de leur pivot —
          avec leur type (Crypto / Action US), date de détection, date d'élimination et badge 📊
          avant résultats. Filtres Tous / Crypto / Actions US et tri par colonne.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Le rôle de l'IA dans la stratégie">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">Candidats journalisés</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">ACTIVE</span>
              </div>
              <p>Le scanner enregistre chaque candidat avec son détail point par point, et les
              trades clôturés avec leur verdict en R — la matière première s'accumule dès
              maintenant.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">Catalyseur news · ranking</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">À CADRER · ÉTAPE 6</span>
              </div>
              <p>Évaluer le catalyseur « news » (le point manquant du classement : flux ETF,
              listings, réglementation) et ranker les faux pivots pour écarter les cassures
              qui n'en sont pas.</p>
            </div>
          </div>
        </carte>

        <carte titre="Le fonctionnement">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Définition</text>
            <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cette page · l'étalon</text>
            <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="210" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Candidats</text>
            <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">journalisés · verdicts R</text>
            <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(167,139,250,0.08)" stroke="#a78bfa" stroke-width="1" />
            <text x="350" y="37.5" text-anchor="middle" fill="#a78bfa" font-size="8" font-weight="700">Ollama local</text>
            <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">hors temps réel</text>
            <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="490" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Propositions</text>
            <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">catalyseur · ranking</text>
            <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <p class="text-center whitespace-nowrap">IA locale (Ollama), hors du temps réel — les propositions ne prennent effet qu'après validation du propriétaire. Prompts : Outils IA › Prompts IA.</p>
        </carte>

        <carte titre="Les objectifs">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Évaluer le catalyseur news</div>
              <p>Flux ETF, annonces de listing, réglementation — la lecture qui complète les
              critères chiffrables.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Ranker les faux pivots</div>
              <p>Conviction sur les candidats détectés, pour écarter les cassures qui n'en
              sont pas.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Analyser par pilier</div>
              <p>Quels critères du classement gagnent réellement — pour le recalibrer.</p>
            </div>
          </div>
        </carte>

        <carte titre="Les garde-fous">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
              <!-- La chaîne d'exécution : moteur → trade -->
              <rect x="30" y="22" width="64" height="16" rx="3" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
              <text x="62" y="33.5" text-anchor="middle" fill="#34d399" font-size="6.5" font-weight="700">moteur Rockets</text>
              <line x1="98" y1="30" x2="228" y2="30" stroke="#34d399" stroke-width="1.6" />
              <polygon points="234,30 226,27 226,33" fill="#34d399" />
              <rect x="238" y="22" width="72" height="16" rx="3" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
              <text x="274" y="33.5" text-anchor="middle" fill="#34d399" font-size="7" font-weight="700">exécution</text>
              <!-- L'IA : sous la chaîne, elle observe -->
              <line x1="140" y1="66" x2="140" y2="40" stroke="#a78bfa" stroke-width="1" stroke-dasharray="3 3" />
              <polygon points="140,36 137,42 143,42" fill="#a78bfa" />
              <text x="150" y="54" fill="#a78bfa" font-size="7" font-weight="700">observe</text>
              <circle cx="140" cy="76" r="10" fill="rgba(167,139,250,0.10)" stroke="#a78bfa" stroke-width="1.4" />
              <text x="140" y="79.5" text-anchor="middle" fill="#a78bfa" font-size="7" font-weight="700">IA</text>
              <text x="140" y="102" text-anchor="middle" fill="#a78bfa" font-size="7" font-weight="700">Ollama local</text>
              <!-- Le chemin interdit : vers l'exécution -->
              <line x1="158" y1="68" x2="255" y2="44" stroke="#f87171" stroke-width="1" stroke-dasharray="3 3" />
              <line x1="200" y1="49" x2="210" y2="59" stroke="#f87171" stroke-width="1.6" />
              <line x1="210" y1="49" x2="200" y2="59" stroke="#f87171" stroke-width="1.6" />
              <text x="232" y="64" fill="#f87171" font-size="7" font-weight="700">jamais dans la boucle</text>
            </svg>
            <div class="flex flex-col gap-2">
              <div class="flex flex-wrap gap-2">
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">N'ouvre jamais de trade</span>
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">Aucune autonomie sur l'exécution</span>
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-violet-400 border-violet-400/40 bg-violet-400/10">Conseille, propose</span>
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-white border-white/40 bg-white/10">Réglages : acte du propriétaire</span>
              </div>
              <p>Le moteur applique la définition figée de cette page. Ses propositions
              (catalyseur, véto unlocks, ranking) ne prennent effet qu'après validation du
              propriétaire.</p>
            </div>
          </div>
        </carte>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import LexiquePanel from '@/components/common/LexiquePanel.vue'
import { ref, computed, defineComponent, h, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface ReglagesStrategie { etat: string; capital: number; risque_pct: number }

const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', { class: 'text-xs font-semibold text-blue-400 uppercase tracking-widest mb-2.5', innerHTML: props.titre }),
      h('div', { class: 'text-white text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0' }, slots.default?.()),
    ])
  },
})
const carte = Carte
const Valeur = defineComponent({
  props: { etiquette: { type: String, required: true }, valeur: { type: String, required: true } },
  setup: (p: { etiquette: string; valeur: string }) => () =>
    h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-4 py-3' }, [
      h('div', { class: 'text-[10px] text-white uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
const valeur = Valeur

// ── Onglets (Lexique en onglet, gabarit SMC) ─────────────────────────────────
const onglets = ['Définition', 'Lexique', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Scanner', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const s = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[]).find(x => x.id === 'rockets')
    if (s) reglages.value = s
  } catch { /* registre indisponible */ }
})

const badgeClasse = computed(() =>
  reglages.value?.etat === 'Officielle'
    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
    : 'bg-amber-500/10 text-amber-400 border-amber-500/30')

function reglageStr(champ: 'capital' | 'risque' | 'unR'): string {
  const r = reglages.value
  if (!r) return '—'
  if (champ === 'capital') return r.capital > 0 ? `${r.capital.toLocaleString('fr-FR')} $` : 'à renseigner'
  if (champ === 'risque') return `${r.risque_pct} % (réglable par profil)`
  return r.capital > 0 ? `${(r.capital * r.risque_pct / 100).toLocaleString('fr-FR')} $` : '—'
}
</script>

<script lang="ts">
export default { name: 'RocketsDefinitionView' }
</script>
