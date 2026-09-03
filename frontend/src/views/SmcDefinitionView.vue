<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <!-- En-tête : identité + état du registre -->
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">📐 Les caractéristiques de la stratégie SMC</h1>
      <span
        v-if="reglages"
        class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border"
        :class="badgeClasse"
      >{{ reglages.etat }}</span>
    </div>

    <!-- Barre d'onglets (décision étape 3 : Définition / Décision d'entrée /
         Gestion des trades ouverts / Money management / Lexique en onglet) -->
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t"
        class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-blue-400' : 'text-white border-transparent hover:text-white/70'"
        @click="onglet = t"
      >{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ ONGLET DÉFINITION (composant dédié — schémas SVG) ═══ -->
      <SmcOngletDefinition v-if="onglet === 'Définition'" />

      <!-- ═══ ONGLET DÉCISION D'ENTREE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="Le parcours d'un signal">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <!-- Les 5 stations du parcours, dans l'ordre -->
            <rect x="6" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="50" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Retour à la zone</text>
            <text x="50" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">OB sain · &lt; 8×ATR</text>
            <rect x="122" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="166" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Score + force</text>
            <text x="166" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">seuil · force ≥ 4</text>
            <rect x="238" y="22" width="88" height="24" rx="4" fill="rgba(251,191,36,0.08)" stroke="#fbbf24" stroke-width="1" />
            <text x="282" y="37.5" text-anchor="middle" fill="#fbbf24" font-size="8" font-weight="700">Qualité de zone</text>
            <text x="282" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cœur · MTF · OTE</text>
            <rect x="354" y="22" width="88" height="24" rx="4" fill="rgba(248,113,113,0.08)" stroke="#f87171" stroke-width="1" />
            <text x="398" y="37.5" text-anchor="middle" fill="#f87171" font-size="8" font-weight="700">Porte de trade</text>
            <text x="398" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">sinon veto</text>
            <rect x="470" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="514" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Signal émis</text>
            <text x="514" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">Telegram · clôture</text>
            <!-- Enchaînement -->
            <line x1="98" y1="34" x2="114" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="118,34 112,31.5 112,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="214" y1="34" x2="230" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="234,34 228,31.5 228,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="330" y1="34" x2="346" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="350,34 344,31.5 344,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="446" y1="34" x2="462" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="466,34 460,31.5 460,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Le retour à la zone</div>
              <p>Le prix revient au bord d'un Order Block haussier ou baissier : zone vierge (jamais
              signalée), état non profond, à moins de 8×ATR.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Le score de zone</div>
              <p>Le score enrichi atteint le seuil de trade et la force minimale (≥ 4/10) ; pour une
              BSZone, score ≥ 7/10.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ La qualité de zone</div>
              <p>Zone cœur (OB ∩ OTE ∩ FVG), confluences multi-timeframe ou Fibonacci OTE : le barème
              par actif décide.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ La porte de trade</div>
              <p>Aucun trade rempli non neutralisé (TP1 non atteint), aucun signal déjà poussé sur la
              barre — sinon veto, rien ne part.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">⑤ L'émission</div>
              <p>Imminence Telegram immédiate, intrabar dès la qualification ; le trade est confirmé à
              la clôture de la barre.</p>
            </div>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="La porte de trade unique">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
              <!-- Axe temporel -->
              <line x1="20" y1="66" x2="414" y2="66" stroke="rgba(255,255,255,0.3)" stroke-width="1" />
              <polygon points="420,66 413,63.5 413,68.5" fill="rgba(255,255,255,0.3)" />
              <!-- La barrière : tant que TP1 n'est pas atteint -->
              <rect x="298.5" y="36" width="3" height="60" rx="1.5" fill="#f87171" />
              <circle cx="300" cy="33" r="3" fill="#f87171" />
              <!-- Nouveau setup : stoppé net -->
              <circle cx="230" cy="66" r="4.5" fill="#34d399" />
              <line x1="226" y1="62" x2="234" y2="70" stroke="#f87171" stroke-width="1.6" />
              <line x1="234" y1="62" x2="226" y2="70" stroke="#f87171" stroke-width="1.6" />
              <text x="230" y="50" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">nouveau setup</text>
              <text x="230" y="86" text-anchor="middle" fill="#e5e7eb" font-size="7">bloqué</text>
              <!-- Trade ouvert : la porte reste fermée -->
              <circle cx="370" cy="66" r="4.5" fill="#ffffff" />
              <text x="370" y="50" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">trade rempli</text>
              <text x="370" y="86" text-anchor="middle" fill="#f87171" font-size="7">TP1 non atteint</text>
            </svg>
            <p>Interdiction d'ouvrir un nouveau trade tant qu'un trade existant est rempli sans avoir
            atteint TP1 (ou son break-even). Un ordre en attente — non rempli — ne bloque pas, et une
            seule émission est possible par barre confirmée, tous moteurs confondus. Avec la règle de
            l'un-signal : une zone ne signale qu'une fois.</p>
          </carte>
          <carte titre="Les niveaux consommés">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
              <!-- Avant : niveau de liquidité vivant -->
              <line x1="20" y1="44" x2="190" y2="44" stroke="#fbbf24" stroke-width="1.2" stroke-dasharray="5 3" />
              <text x="24" y="37" fill="#fbbf24" font-size="8" font-weight="700">PDH</text>
              <polyline points="20,92 60,62 85,74 130,36 150,70 185,56" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <circle cx="130" cy="36" r="4.5" fill="none" stroke="#f87171" stroke-width="1.4" />
              <text x="106" y="25" fill="#f87171" font-size="8" font-weight="700">sweep</text>
              <!-- Transition -->
              <line x1="198" y1="44" x2="214" y2="44" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
              <polygon points="218,44 212,41.5 212,46.5" fill="rgba(255,255,255,0.4)" />
              <!-- Après : niveau rayé, hors carnet -->
              <line x1="224" y1="44" x2="420" y2="44" stroke="rgba(255,255,255,0.22)" stroke-width="1.2" stroke-dasharray="5 3" />
              <line x1="314" y1="38" x2="326" y2="50" stroke="#f87171" stroke-width="1.6" />
              <line x1="326" y1="38" x2="314" y2="50" stroke="#f87171" stroke-width="1.6" />
              <text x="288" y="27" fill="#e5e7eb" font-size="8" font-weight="700">consommé</text>
              <text x="338" y="60" fill="#e5e7eb" font-size="7">disparaît du carnet</text>
            </svg>
            <p>Tout niveau de liquidité touché sur barre confirmée — par sweep (mèche qui perce puis
            referme) ou par cassure franche — est consommé : il disparaît du carnet et ne peut plus
            servir de cible ni de confluence.</p>
          </carte>
        </div>

        <carte titre="Exécution — le retest limite">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
              <!-- Zone : Order Block -->
              <rect x="30" y="58" width="70" height="22" fill="rgba(52,211,153,0.15)" stroke="#34d399" stroke-width="1" />
              <text x="78" y="78" fill="#34d399" font-size="8" font-weight="700">OB</text>
              <!-- Prix : impulsion, retour au bord, remplissage, suite -->
              <polyline points="30,74 110,62 180,20 260,54 320,58 350,38 404,24" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
              <polygon points="410,22 400,25 406,31" fill="#34d399" />
              <!-- Ordre limite posé au bord de la zone -->
              <line x1="200" y1="58" x2="420" y2="58" stroke="#ffffff" stroke-width="0.9" stroke-dasharray="4 3" />
              <text x="150" y="52" fill="#e5e7eb" font-size="8" font-weight="700">en attente</text>
              <circle cx="320" cy="58" r="3" fill="#ffffff" />
              <text x="292" y="76" fill="#ffffff" font-size="8" font-weight="700">rempli au retest</text>
              <text x="330" y="30" fill="#34d399" font-size="8" font-weight="700">impulsion</text>
            </svg>
            <div class="flex flex-col gap-2">
              <p><b class="text-white">L'ordre est posé, pas le trade.</b> L'entrée est placée en
              limite au bord de la zone — haut de l'OB pour un achat, bas pour une vente. Le trade
              n'est <b class="text-white">rempli</b> que si le prix revient toucher l'entrée.</p>
              <p>Ce modèle « retest limite » a gagné l'A/B <b class="text-white">15/15</b> contre
              l'entrée au marché à la cassure — il est figé dans la stratégie. Pas de retour du prix, pas
              de trade.</p>
            </div>
          </div>
        </carte>
      </div>

      <!-- ═══ ONGLET GESTION (composant dédié — ventes partielles) ═══ -->
      <SmcOngletGestion v-if="onglet === 'Gestion des trades ouverts'" />

      <!-- ═══ ONGLET MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Les trois couches">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Conventions par actif</div>
              <p>Taille du pip, valeur du pip, lot min/max : la grammaire commune de tous les
              calculs — réglée dans l'onglet gestion du risque.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Allocation par stratégie</div>
              <p>Capital dédié et risque de 1 à 3 % par trade, réglés dans Paramètres ›
              stratégies › SMC.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Calcul à l'émission</div>
              <p>Le lot sort de la formule ci-dessous, appliquée au moment du signal — jamais
              avant.</p>
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
          <p class="text-center">Le risque en euros est figé à l'émission : seul le lot s'adapte à
          la distance du stop de la zone.</p>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par trade" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>

        <carte titre="Le R, unité de compte">
          <svg viewBox="0 0 560 80" class="w-full aspect-[560/80] mb-2">
            <!-- Échelle des verdicts, en multiples du risque -->
            <rect x="20" y="46" width="160" height="6" fill="rgba(248,113,113,0.25)" />
            <rect x="180" y="46" width="360" height="6" fill="rgba(52,211,153,0.25)" />
            <line x1="20" y1="40" x2="540" y2="40" stroke="rgba(255,255,255,0.4)" stroke-width="1" />
            <line x1="80" y1="28" x2="80" y2="52" stroke="#f87171" stroke-width="1.6" />
            <text x="80" y="70" text-anchor="middle" fill="#f87171" font-size="8" font-weight="700">SL −1R</text>
            <line x1="180" y1="28" x2="180" y2="52" stroke="#ffffff" stroke-width="1.6" />
            <text x="180" y="70" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">BE 0</text>
            <line x1="240" y1="28" x2="240" y2="52" stroke="#60a5fa" stroke-width="1.6" />
            <text x="240" y="70" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">TP1 +0,6R</text>
            <line x1="380" y1="28" x2="380" y2="52" stroke="#34d399" stroke-width="1.6" />
            <text x="380" y="70" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP2 +2R</text>
            <line x1="480" y1="28" x2="480" y2="52" stroke="#34d399" stroke-width="1.6" />
            <text x="480" y="70" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">TP3</text>
            <text x="280" y="18" text-anchor="middle" fill="#e5e7eb" font-size="7" font-weight="700">1R = le risque initial du trade</text>
          </svg>
          <p>Tous les trades se mesurent en multiples du risque initial : la performance se lit en
          R cumulé — indépendante du capital, homogène entre actifs — et se convertit en évolution
          du capital via le risque par trade. TP3 garde sa distance réelle : celle de la liquidité
          visée.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET LEXIQUE (le panneau du lexique SMC, en page) ═══ -->
      <div v-if="onglet === 'Lexique'" class="flex flex-col gap-3">
        <LexiquePanel source="smc" />
      </div>

      <!-- ═══ ONGLET ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Le rôle de l'IA dans la stratégie">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">Analyse stratégique</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">ACTIVE</span>
              </div>
              <p>Lit les signaux clôturés (bouton « Analyse SMC » du graphique), évalue la
              performance par type de confluence et par contexte, produit des recommandations
              lisibles.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="flex items-center gap-2 mb-1">
                <div class="font-semibold">Filtre temps réel &amp; monitoring</div>
                <span class="px-2 py-0.5 rounded-full border text-[10px] font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">À CADRER · ÉTAPE 6</span>
              </div>
              <p>Cahier des charges discuté et acté avant tout branchement. Premier chantier
              tracé en roadmap : la conviction IA à l'émission, en observation seule.</p>
            </div>
          </div>
        </carte>

        <carte titre="Le fonctionnement">
          <div class="flex flex-col items-center gap-3">
            <svg viewBox="0 0 560 70" class="w-full aspect-[560/70]">
            <rect x="26" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="70" y="37.5" text-anchor="middle" fill="#ffffff" font-size="8" font-weight="700">Définition</text>
            <text x="70" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">cette page · l'étalon</text>
            <rect x="166" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="210" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Trades clôturés</text>
            <text x="210" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">historique complet</text>
            <rect x="306" y="22" width="88" height="24" rx="4" fill="rgba(167,139,250,0.08)" stroke="#a78bfa" stroke-width="1" />
            <text x="350" y="37.5" text-anchor="middle" fill="#a78bfa" font-size="8" font-weight="700">Ollama local</text>
            <text x="350" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">hors temps réel</text>
            <rect x="446" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="490" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Analyse</text>
            <text x="490" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">recommandations</text>
            <line x1="118" y1="34" x2="158" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="162,34 156,31.5 156,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="258" y1="34" x2="298" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="302,34 296,31.5 296,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="398" y1="34" x2="438" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="442,34 436,31.5 436,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
            <p class="text-center whitespace-nowrap">L'IA tourne en local (Ollama) et intervient hors du temps réel : sur demande, jamais dans la boucle de décision d'un signal. Les textes des prompts se règlent dans Outils IA › Prompts IA.</p>
          </div>
        </carte>

        <carte titre="Les objectifs">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Expliquer la performance</div>
              <p>Quelles confluences gagnent, sur quels actifs, sur quelles plages horaires —
              et pourquoi.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Détecter les dérives</div>
              <p>Signaux hors définition, contextes perdants récurrents : le moteur doit rester
              fidèle à sa définition.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Évaluer avant d'étendre</div>
              <p>Un filtre temps réel ne se branchera que si la preuve le justifie — décision
              de l'étape 6.</p>
            </div>
          </div>
        </carte>

        <carte titre="Les garde-fous">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-center">
            <svg viewBox="0 0 440 110" class="w-full aspect-[440/110]">
              <!-- La chaîne d'exécution : moteur → trade -->
              <rect x="30" y="22" width="64" height="16" rx="3" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
              <text x="62" y="33.5" text-anchor="middle" fill="#34d399" font-size="7" font-weight="700">moteur SMC</text>
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
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">Aucune autonomie sur les seuils</span>
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-violet-400 border-violet-400/40 bg-violet-400/10">Conseille, explique</span>
                <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-white border-white/40 bg-white/10">Réglages : acte du propriétaire</span>
              </div>
              <p>Le moteur SMC applique la définition figée — l'étalon est le Pine. L'IA conseille
              et explique ; toute modification de réglage est un acte du propriétaire dans les
              Paramètres.</p>
            </div>
          </div>
        </carte>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import LexiquePanel from '@/components/common/LexiquePanel.vue'
import SmcOngletDefinition from '@/components/smc/SmcOngletDefinition.vue'
import SmcOngletGestion from '@/components/smc/SmcOngletGestion.vue'
import { Carte as carte, Valeur as valeur } from '@/components/common/carteTitree'
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface ReglagesStrategie {
  etat: string; capital: number; risque_pct: number
}



// ── Onglets (décision étape 3 : Définition première page + Lexique en onglet)
const onglets = ['Définition', 'Lexique', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

// ── Réglages live de la stratégie (registre) ─────────────────────────────────
const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const smc = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[])
      .find(s => s.id === 'SMC')
    if (smc) reglages.value = smc
  } catch { /* registre indisponible — badge masqué */ }
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
export default { name: 'SmcDefinitionView' }
</script>
