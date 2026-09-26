<template>
  <!-- Toile de fond thématisée des interrupteurs du pedestal (owner 25/09) :
       une scène filaire blanche très discrète posée derrière le contenu de
       la fenêtre digitale — journal, rapport, cerveau, courbe, base de
       données. Purement décorative : pointer-events none, opacité faible. -->
  <svg
    viewBox="0 0 120 96" preserveAspectRatio="xMidYMid meet"
    class="absolute inset-0 w-full h-full pointer-events-none text-white/[0.07]"
    fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"
    aria-hidden="true"
  >
    <!-- presse : une page de journal -->
    <template v-if="theme === 'presse'">
      <rect x="8" y="8" width="104" height="80" rx="2" />
      <line x1="8" y1="22" x2="112" y2="22" />
      <line x1="44" y1="22" x2="44" y2="88" />
      <line x1="78" y1="22" x2="78" y2="88" />
      <line x1="12" y1="14" x2="36" y2="14" stroke-width="3" />
      <line x1="48" y1="28" x2="74" y2="28" /><line x1="48" y1="34" x2="74" y2="34" /><line x1="48" y1="40" x2="70" y2="40" />
      <line x1="82" y1="28" x2="108" y2="28" /><line x1="82" y1="34" x2="108" y2="34" /><line x1="82" y1="40" x2="104" y2="40" /><line x1="82" y1="46" x2="108" y2="46" />
      <line x1="12" y1="28" x2="40" y2="28" /><line x1="12" y1="34" x2="40" y2="34" /><line x1="12" y1="40" x2="40" y2="40" /><line x1="12" y1="46" x2="38" y2="46" /><line x1="12" y1="52" x2="40" y2="52" />
    </template>

    <!-- rapport : feuilles de rapport — histogramme + courbe -->
    <template v-else-if="theme === 'rapport'">
      <line x1="10" y1="86" x2="110" y2="86" />
      <line x1="10" y1="10" x2="10" y2="86" />
      <rect x="20" y="60" width="12" height="26" />
      <rect x="38" y="44" width="12" height="42" />
      <rect x="56" y="52" width="12" height="34" />
      <rect x="74" y="28" width="12" height="58" />
      <rect x="92" y="38" width="12" height="48" />
      <polyline points="20,40 40,30 60,36 80,16 100,22" stroke-width="2" />
    </template>

    <!-- ia : cerveau + ramifications de bulles d'idées -->
    <template v-else-if="theme === 'ia'">
      <path d="M46 24 C30 22 22 34 26 46 C20 56 28 70 42 70 L46 78 C60 82 74 76 74 64 C88 62 92 48 84 40 C88 28 76 18 64 22 C58 14 44 14 42 24 Z" />
      <circle cx="30" cy="14" r="5" /><circle cx="96" cy="22" r="4" /><circle cx="102" cy="56" r="5" />
      <circle cx="20" cy="66" r="4" /><circle cx="60" cy="6" r="4" />
      <line x1="33" y1="17" x2="44" y2="23" /><line x1="92" y1="24" x2="78" y2="31" /><line x1="97" y1="52" x2="84" y2="50" />
      <line x1="24" y1="63" x2="34" y2="59" /><line x1="58" y1="10" x2="56" y2="20" />
    </template>

    <!-- graphiques : courbe de prix -->
    <template v-else-if="theme === 'graphiques'">
      <polyline points="8,70 24,60 36,66 48,44 62,52 74,30 88,38 100,18 112,26" stroke-width="2.5" />
      <line x1="8" y1="86" x2="112" y2="86" stroke-dasharray="3 4" />
    </template>

    <!-- donnees : base de données (cylindre) -->
    <template v-else>
      <ellipse cx="60" cy="22" rx="34" ry="10" />
      <path d="M26 22 V70 C26 76 41 80 60 80 C79 80 94 76 94 70 V22" />
      <path d="M26 38 C26 44 41 48 60 48 C79 48 94 44 94 38" />
      <path d="M26 54 C26 60 41 64 60 64 C79 64 94 60 94 54" />
    </template>
  </svg>
</template>

<script setup lang="ts">
defineProps<{ theme: 'presse' | 'rapport' | 'ia' | 'graphiques' | 'donnees' }>()
</script>
