<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <!-- En-tête : identité + état du registre -->
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">📐 Les caractéristiques de la stratégie Straddle</h1>
      <span
        v-if="reglages"
        class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border"
        :class="badgeClasse"
      >{{ reglages.etat }}</span>
    </div>

    <!-- Barre d'onglets -->
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t"
        class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-amber-400' : 'text-white border-transparent hover:text-white/70'"
        @click="onglet = t"
      >{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ ONGLET DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
            <!-- Prix de référence E -->
            <line x1="20" y1="55" x2="420" y2="55" stroke="#ffffff" stroke-width="0.8" stroke-dasharray="4 3" />
            <text x="22" y="50" fill="#ffffff" font-size="7" font-weight="700">E</text>
            <!-- L'annonce -->
            <line x1="200" y1="15" x2="200" y2="100" stroke="#fbbf24" stroke-width="1" stroke-dasharray="3 3" />
            <text x="150" y="12" fill="#fbbf24" font-size="7" font-weight="700">annonce</text>
            <!-- Range avant l'annonce -->
            <polyline points="20,58 50,50 80,60 110,52 140,58 170,53 200,55" fill="none" stroke="#e5e7eb" stroke-width="1.2" stroke-linejoin="round" />
            <!-- La cassure remplit la jambe du bon côté -->
            <line x1="200" y1="55" x2="285" y2="18" stroke="#34d399" stroke-width="2" />
            <polygon points="285,18 277,21 282,27" fill="#34d399" />
            <circle cx="250" cy="42" r="2.8" fill="#ffffff" />
            <text x="295" y="30" fill="#34d399" font-size="7" font-weight="700">jambe remplie</text>
            <!-- La jambe opposée est annulée -->
            <line x1="230" y1="60" x2="230" y2="88" stroke="#f87171" stroke-width="1.2" stroke-dasharray="3 3" />
            <polygon points="230,92 226,84 234,84" fill="#f87171" />
            <line x1="226" y1="66" x2="234" y2="74" stroke="#f87171" stroke-width="1.4" />
            <line x1="234" y1="66" x2="226" y2="74" stroke="#f87171" stroke-width="1.4" />
            <text x="244" y="86" fill="#f87171" font-size="7" font-weight="700">annulée (OCO)</text>
          </svg>
          Le straddle capture les mouvements de volatilité violents et brefs déclenchés par les
          événements de marché : autour d'une annonce forte, deux ordres miroir attendent la
          cassure — la jambe du bon côté entre, quelle que soit la direction, l'autre est
          annulée. Pas d'anticipation directionnelle : la stratégie achète la volatilité
          elle-même.
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le déclencheur">
            Périmètre acté : <b class="text-white">XAU, BTC, NAS100, SP500 et DAX</b>. Les annonces
            US fortes (impact High du calendrier économique) arment XAU, BTC, NAS100 et SP500 ;
            le DAX joue <b class="text-white">l'ouverture européenne</b> (9h00 Paris). XAU et BTC
            en ticks temps réel, NAS100, SP500 et DAX en M1 via MT5.
          </carte>
          <carte titre="La fenêtre">
            À T-30 minutes, le moteur entre en préparation : observation du range et chauffage de
            l'ATR14. À T-10 secondes (réglable), les deux jambes sont posées au prix courant E
            et armées immédiatement — le premier franchissement de E, même avant l'annonce,
            remplit la jambe dans son sens.
          </carte>
        </div>
      </div>

      <!-- ═══ ONGLET LEXIQUE ═══ -->
      <div v-if="onglet === 'Lexique'" class="flex flex-col gap-3">
        <LexiquePanel source="straddle" />
      </div>

      <!-- ═══ ONGLET DÉCISION D'ENTRÉE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="L'ordre miroir">
          <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
            <!-- Prix E : les deux ordres au même niveau -->
            <line x1="20" y1="55" x2="420" y2="55" stroke="#ffffff" stroke-width="0.8" stroke-dasharray="4 3" />
            <text x="24" y="50" fill="#ffffff" font-size="7" font-weight="700">E · prix à T-10 s</text>
            <!-- Buy-stop au-dessus de E -->
            <line x1="220" y1="50" x2="220" y2="20" stroke="#34d399" stroke-width="1.6" />
            <polygon points="220,14 215,22 225,22" fill="#34d399" />
            <text x="232" y="26" fill="#34d399" font-size="7" font-weight="700">buy-stop</text>
            <!-- Sell-stop en dessous de E -->
            <line x1="220" y1="60" x2="220" y2="90" stroke="#f87171" stroke-width="1.6" />
            <polygon points="220,96 215,88 225,88" fill="#f87171" />
            <text x="232" y="80" fill="#f87171" font-size="7" font-weight="700">sell-stop</text>
            <!-- Le lien OCO -->
            <text x="220" y="107" text-anchor="middle" fill="#fbbf24" font-size="7" font-weight="700">OCO — la première remplie annule l'autre</text>
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Même prix</div>
              <p>Les deux jambes entrent au même prix E : le prix courant à T-10 secondes.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Buy-stop</div>
              <p>À E : se remplit si le prix passe au-dessus.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Sell-stop</div>
              <p>À E : se remplit si le prix passe en dessous.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ OCO</div>
              <p>La première jambe remplie annule l'autre instantanément.</p>
            </div>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le risque R — l'échelle en R">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
              <!-- Échelle des niveaux, proportionnelle en R -->
              <line x1="68" y1="22" x2="290" y2="22" stroke="#34d399" stroke-width="0.9" stroke-dasharray="4 3" />
              <text x="298" y="25" fill="#34d399" font-size="7.5" font-weight="700">TP2 · +2R</text>
              <line x1="68" y1="46" x2="290" y2="46" stroke="#60a5fa" stroke-width="0.9" stroke-dasharray="4 3" />
              <text x="298" y="49" fill="#60a5fa" font-size="7.5" font-weight="700">TP1 · +1R</text>
              <line x1="68" y1="70" x2="290" y2="70" stroke="#ffffff" stroke-width="0.9" stroke-dasharray="4 3" />
              <text x="298" y="73" fill="#ffffff" font-size="7.5" font-weight="700">E · 0R</text>
              <line x1="68" y1="94" x2="290" y2="94" stroke="#f87171" stroke-width="0.9" stroke-dasharray="4 3" />
              <text x="298" y="97" fill="#f87171" font-size="7.5" font-weight="700">SL · −1R</text>
              <!-- Remplissage, TP1 (BE), TP2 (trailing) -->
              <polyline points="30,70 55,82 85,76 150,46 175,60 220,22 255,14" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <circle cx="150" cy="46" r="2.6" fill="#ffffff" />
              <circle cx="220" cy="22" r="2.6" fill="#ffffff" />
              <text x="232" y="48" fill="#fbbf24" font-size="7" font-weight="700">trailing</text>
            </svg>
            <p>R, l'unité de risque, vaut la distance du stop : <b class="text-white">R = facteur
            SL × ATR14</b> (réglable dans la carte paramètres — le lot en découle : 1R =
            capital × risque). Tous les niveaux s'expriment en R : SL à ±1R, TP1 à 1R, TP2 à 2R.</p>
          </carte>
          <carte titre="L'expiration">
            Si aucun franchissement ne survient, les deux jambes expirent 30 minutes après
            l'annonce — la volatilité attendue n'est pas venue, on repart sans position.
          </carte>
        </div>
      </div>

      <!-- ═══ ONGLET GESTION DES TRADES OUVERTS ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="Le cycle de la jambe survivante — moteur unifié">
          <p class="text-[11px] text-white leading-relaxed mb-3">
            La jambe survivante vit dans le <b class="text-white">même moteur de gestion que la
            SMC</b> (crate <b class="text-white">gestion_trades</b>) : mêmes règles de SL/BE/TP/
            trailing/expiration, nourries au tick. La genèse reste straddle : 2 jambes miroir à E
            (timer T-10 s), OCO, R = facteur SL × ATR H1.
          </p>
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="6" y="22" width="76" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="44" y="37.5" text-anchor="middle" fill="#ffffff" font-size="7.5" font-weight="700">2 jambes à E</text>
            <text x="44" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6">OCO · SL E∓1R</text>
            <rect x="146" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="190" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP1 · 1R</text>
            <text x="190" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">SL au tampon E∓0,5R</text>
            <rect x="286" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="330" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">TP2 · 2R</text>
            <text x="330" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">SL à TP1 · trailing</text>
            <rect x="426" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="470" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP3 · 3R</text>
            <text x="470" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cible de la passe</text>
            <line x1="86" y1="34" x2="138" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="142,34 136,31.5 136,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="238" y1="34" x2="278" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="282,34 276,31.5 276,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="378" y1="34" x2="418" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="422,34 416,31.5 416,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Ouverture</div>
              <p>À T-10 s, le timer ouvre les 2 jambes au prix courant E — R = facteur SL ×
              ATR H1, SL = E∓1R, TP1/2/3 = ±1/2/3R.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② TP1 touché</div>
              <p>Le stop passe au <b class="text-white">tampon E∓0,5R</b> (décision 27/08
              anti-whipsaw — le rebond à E ne tue plus la gagnante).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ TP2 touché</div>
              <p>Le stop monte à TP1 et le trailing démarre — au tick, distance ×R
              réglable, jamais vers l'arrière.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ TP3 · 3R</div>
              <p>La cible de la passe : le mouvement complet est capturé si le spike
              atteint +3R.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">⑤ Sortie</div>
              <p>TP3, trailing touché (TS), retour au stop ou expiration 60 min — au
              premier des quatre.</p>
            </div>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Les verdicts">
            <div class="flex flex-wrap gap-2 mb-2">
              <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">SL · −1R</span>
              <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-white border-white/40 bg-white/10">BE · 0</span>
              <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TS</span>
              <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TP3 · 2R net</span>
              <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">Expire</span>
            </div>
            <p>Chaque passe clôturée reçoit son verdict et son <b class="text-white">R net</b>
            (survivante + jambe morte — le SL de la perdante est déduit). Avec la comptabilité
            « TP acquis » du moteur unifié, une jambe qui touche TP1 puis expire vaut +1R :
            une passe où la survivante touche TP1 ne peut plus être nette négative. En
            Observation : journalisé, silencieux sur Telegram.</p>
          </carte>
          <carte titre="L'expiration">
            <div class="grid grid-cols-2 gap-2 mb-3">
              <valeur etiquette="Après l'ouverture" valeur="60 min" />
              <valeur etiquette="Verdict" valeur="Expire" />
            </div>
            <p>Au-delà de 60 minutes, la jambe expire — avec la comptabilité « TP acquis » :
            TP1 touché = +1R (le palier est acquis), sinon 0. La passe ne survit pas à sa
            fenêtre d'événement.</p>
          </carte>
        </div>
      </div>

      <!-- ═══ ONGLET MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Les trois couches">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Conventions par actif</div>
              <p>Taille et valeur du pip, lot min/max — réglées dans l'onglet gestion du
              risque.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Allocation par stratégie</div>
              <p>Capital dédié et risque de 1 à 3 % par passe, réglés dans Paramètres ›
              stratégies › Straddle.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Calcul au signal</div>
              <p>Le lot sort de la formule ci-dessous ; stop = 1R = facteur SL × ATR14.</p>
            </div>
          </div>
        </carte>

        <carte titre="La formule du lot">
          <div class="flex flex-wrap items-center justify-center gap-x-3 gap-y-2 font-mono text-xl py-3">
            <span class="text-violet-400 font-bold">lot</span>
            <span class="text-white">=</span>
            <span class="text-white">(</span>
            <span class="font-bold">capital</span>
            <span class="text-white">×</span>
            <span class="text-blue-400 font-bold">risque %</span>
            <span class="text-white">) ÷ (</span>
            <span class="text-amber-400 font-bold">stop</span>
            <span class="text-white">×</span>
            <span class="text-emerald-400 font-bold">valeur du pip</span>
            <span class="text-white">)</span>
          </div>
          <p class="text-center">Le risque en euros est figé au signal : seul le lot s'adapte à
          la distance du stop, dérivée de l'ATR14.</p>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par passe" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>

        <carte titre="Le R, unité de compte — une passe à la fois">
          <svg viewBox="0 0 560 80" class="w-full aspect-[560/80] mb-2">
            <!-- Échelle des verdicts, en multiples du risque -->
            <rect x="20" y="46" width="160" height="6" fill="rgba(248,113,113,0.25)" />
            <rect x="180" y="46" width="360" height="6" fill="rgba(52,211,153,0.25)" />
            <line x1="20" y1="40" x2="540" y2="40" stroke="rgba(255,255,255,0.4)" stroke-width="1" />
            <line x1="80" y1="28" x2="80" y2="52" stroke="#f87171" stroke-width="1.6" />
            <text x="80" y="70" text-anchor="middle" fill="#f87171" font-size="8" font-weight="700">SL −1R</text>
            <line x1="180" y1="28" x2="180" y2="52" stroke="#ffffff" stroke-width="1.6" />
            <text x="180" y="70" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">BE 0</text>
            <line x1="280" y1="28" x2="280" y2="52" stroke="#60a5fa" stroke-width="1.6" />
            <text x="280" y="70" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">TP1 +1R</text>
            <line x1="480" y1="28" x2="480" y2="52" stroke="#34d399" stroke-width="1.6" />
            <text x="480" y="70" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP2 +2R</text>
            <text x="280" y="18" text-anchor="middle" fill="#e5e7eb" font-size="7" font-weight="700">1R = facteur SL × ATR14</text>
          </svg>
          <p>Tous les trades se mesurent en multiples du risque initial : la performance se lit
          en R cumulé et se convertit en évolution du capital via le risque par passe. Le moteur
          ne gère qu'une position par annonce et par actif — jamais de cumul : la volatilité
          événementielle se joue un événement à la fois.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Le rôle de l'IA dans la stratégie">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">La matière première</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">ACTIVE</span>
              </div>
              <p>L'Observation journalise chaque passe dès maintenant — annonce, range,
              remplissage, verdict en R. C'est en accumulant ces passes que l'IA apprendra
              quels événements valent le coup.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">Agenda &amp; minutage par actif</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">À CADRER · ÉTAPE 6</span>
              </div>
              <p>Repérer les événements qui déplacent réellement chaque actif (au-delà du
              libellé « High ») et proposer un minutage d'entrée par type d'événement, sur
              preuve historique. D'autres usages seront définis par le propriétaire.</p>
            </div>
          </div>
        </carte>

        <carte titre="Le fonctionnement">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Définition</text>
            <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cette page · l'étalon</text>
            <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="210" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Passes journalisées</text>
            <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">annonces · verdicts R</text>
            <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(167,139,250,0.08)" stroke="#a78bfa" stroke-width="1" />
            <text x="350" y="37.5" text-anchor="middle" fill="#a78bfa" font-size="8" font-weight="700">Ollama local</text>
            <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">hors temps réel</text>
            <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="490" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Propositions</text>
            <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">agenda · minutage</text>
            <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <p class="text-center whitespace-nowrap">L'IA tourne en local (Ollama), hors du temps réel — ses propositions ne prennent effet qu'après validation du propriétaire. Prompts : Outils IA › Prompts IA.</p>
        </carte>

        <carte titre="Les objectifs">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Un agenda par actif</div>
              <p>Pondéré par l'impact réel mesuré — pas seulement le libellé « High » du
              calendrier.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Un minutage d'entrée</div>
              <p>Recommandé par événement et par actif, sur preuve historique.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Benchmark local</div>
              <p>Le meilleur modèle Ollama pour ce travail — décision de l'étape 6.</p>
            </div>
          </div>
        </carte>

        <carte titre="Les garde-fous">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
              <!-- La chaîne d'exécution : moteur → trade -->
              <rect x="30" y="22" width="64" height="16" rx="3" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
              <text x="62" y="33.5" text-anchor="middle" fill="#34d399" font-size="6.5" font-weight="700">moteur Straddle</text>
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
              <p>Le moteur applique la définition figée de cette page : deux jambes à E, OCO,
              trailing. Les propositions de l'IA (agenda, minutage) ne prennent effet qu'après
              validation du propriétaire dans les réglages.</p>
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

interface ReglagesStrategie {
  etat: string; capital: number; risque_pct: number
}

// ── Mini-composants locaux (gabarit SMC) ─────────────────────────────────────
const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', {
        class: 'text-xs font-semibold text-amber-400 uppercase tracking-widest mb-2.5',
        innerHTML: props.titre,
      }),
      h('div', {
        class: 'text-white text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0',
      }, slots.default?.()),
    ])
  },
})
const carte = Carte

const Valeur = defineComponent({
  props: {
    etiquette: { type: String, required: true },
    valeur: { type: String, required: true },
  },
  setup: (p: { etiquette: string; valeur: string }) => () =>
    h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-4 py-3' }, [
      h('div', { class: 'text-[10px] text-white uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
const valeur = Valeur

// ── Onglets (Lexique en onglet, gabarit SMC) ─────────────────────────────────
const onglets = ['Définition', 'Lexique', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const s = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[])
      .find(x => x.id === 'straddle')
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
  if (champ === 'risque') return `${r.risque_pct} %`
  return r.capital > 0 ? `${(r.capital * r.risque_pct / 100).toLocaleString('fr-FR')} $` : '—'
}
</script>

<script lang="ts">
export default { name: 'StraddleDefinitionView' }
</script>
