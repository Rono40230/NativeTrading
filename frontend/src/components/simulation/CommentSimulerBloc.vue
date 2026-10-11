<template>
  <!-- ❓ Comment simuler (owner 11/10) — mode d'emploi repliable, commun aux
       3 labos. Absorbe l'ancienne note « limite méthodologique ». -->
  <details class="glass-card px-4 py-3">
    <summary class="entete-detail">❓ Comment simuler — le mode d'emploi (6 étapes)</summary>
    <div class="mt-3 grid grid-cols-1 lg:grid-cols-3 gap-4">
      <ol class="col-span-2 flex flex-col gap-2 text-xs text-white/85 list-decimal list-inside leading-relaxed">
        <li><b>Lire le vécu</b> (colonne de gauche) : c'est la référence officielle — capital, Σ R, WR et les paramètres <b>en production actuellement</b>.</li>
        <li><b>Régler les paramètres virtuels</b> (colonne centrale) — ou ⧉ copier les paramètres du vécu puis ajuster. Rien n'est écrit tant qu'on n'applique pas.</li>
        <li><b>Choisir le périmètre</b> avec les chips « Paires simulées » : vide = tous les assets armés · <b>une seule chip = mode PAR ASSET</b> (l'advisory ne compare alors que les essais de cet asset — pensez à relancer le balayage dans ce mode pour créer des essais étiquetés).</li>
        <li><b>▶ Lancer la simulation</b> (~35 s) : re-jeu exact du moteur sur les trades réellement pris → comparatif côte à côte.</li>
        <li><b>Balayer</b> pour cartographier : fractions (quelques minutes) · k du trailing (~3 min, SMC). Chaque essai est conservé dans la bibliothèque (clic = recharge ses paramètres).</li>
        <li><b>🎯 Advisory → ⚡ Activer</b> : écrit la config gagnante — <b>globale</b> (aucune chip) ou <b>surcharge de l'asset seul</b> (1 chip) avec écriture relue-vérifiée et hot-reload ≤ 60 s sur ses seuls couples.</li>
      </ol>
      <div class="rounded-lg border border-amber-500/25 bg-amber-500/5 p-2.5 text-[10px] text-white/80 leading-relaxed">
        <p class="font-bold uppercase tracking-wider text-amber-300 mb-1.5">Garde-fous</p>
        <p>· La simulation rejoue les <b>trades réellement pris</b> : elle ne peut pas inventer les trades qu'un autre réglage aurait fait naître.</p>
        <p class="mt-1">· Règle des 30 <b>par tranche et par asset</b> : sous 30 trades, rien de significatif.</p>
        <p class="mt-1">· <b>Le vécu ne change jamais</b> : les niveaux restent figés à l'émission ; un réglage n'agit que sur les futurs signaux.</p>
      </div>
    </div>
  </details>
</template>

<style scoped>
.entete-detail { font-size: .75rem; font-weight: 600; color: #fff; user-select: none; list-style: none; cursor: pointer; }
.entete-detail::-webkit-details-marker { display: none; }
.entete-detail::before { content: '▸ '; color: rgba(255,255,255,.5); display: inline-block; transition: transform .15s; }
details[open] > .entete-detail::before { transform: rotate(90deg); }
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
