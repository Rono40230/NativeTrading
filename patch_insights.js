const fs = require('fs');

let content = fs.readFileSync('frontend/src/views/MlInsightsView.vue', 'utf8');

content = content.replace(
  /        <!-- SECTION 1 : L'État des Cerveaux -->[\s\S]*?<!-- LIGNE 2 : Prescriptions & Seuils ML -->/g,
  `        <!-- SECTION 1 : L'État des Cerveaux -->
        <section class="glass-card p-4 flex flex-col gap-3 rounded-xl border bg-white/5 border-blue-500/30">
          <div class="flex items-center justify-between shrink-0">
            <h2 class="font-bold flex items-center gap-2 text-base text-blue-400">
              <span>🧠</span> 1. État des Modèles ML
            </h2>
            <button
              class="shrink-0 px-3 py-1 rounded font-semibold text-[10px] uppercase transition-colors shadow-lg border"
              :class="store.retrainState?.en_cours
                ? 'bg-gray-700/50 text-gray-400 border-gray-600/50 cursor-not-allowed'
                : 'bg-blue-600/20 text-blue-300 border-blue-500/30 hover:bg-blue-600/30'"
              :disabled="store.retrainState?.en_cours"
              @click="store.declencherRetrain()"
            >
              {{ store.retrainState?.en_cours ? '⏳ En cours…' : '🔁 Entraînement' }}
            </button>
          </div>
          <div class="flex flex-col gap-3 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <MlRetrainPanel />
          </div>
        </section>

        <!-- SECTION 2 : Performances -->
        <section class="glass-card p-4 flex flex-col gap-3 rounded-xl border bg-white/5 border-emerald-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-emerald-400 shrink-0">
            <span>📈</span> 2. Performances en direct
          </h2>
          <div class="flex flex-col gap-2 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <div v-if="!store.analyse" class="bg-black/20 rounded-lg border border-white/10 p-4 text-center text-gray-400 text-xs h-full flex items-center justify-center">
              Aucune donnée disponible.
            </div>
            <div v-else class="space-y-2">
              <!-- SMC -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-blue-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-blue-200 text-xs">SMC</span>
                    <span class="text-[9px] text-gray-500">({{ store.analyse.smc?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.smc?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.smc?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-gray-300">{{ store.analyse.smc?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.smc?.ml_correlation?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-gray-400">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-gray-300'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>

              <!-- Rockets -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-orange-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-orange-200 text-xs">Rockets</span>
                    <span class="text-[9px] text-gray-500">({{ store.analyse.rockets?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.rockets?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.rockets?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-gray-300">{{ store.analyse.rockets?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.rockets?.conviction_llm?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-gray-400">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-gray-300'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>

              <!-- Straddle -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-purple-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-purple-200 text-xs">Straddle</span>
                    <span class="text-[9px] text-gray-500">({{ store.analyse.straddle?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.straddle?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.straddle?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-gray-300">{{ store.analyse.straddle?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.straddle?.score_llm?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-gray-400">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-gray-300'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

      </div>

      <!-- LIGNE 2 : Prescriptions & Seuils ML -->`
);

content = content.replace(
  /        <!-- SECTION 3 : Prescriptions -->[\s\S]*?<!-- SECTION 4 : Seuils ML -->/,
  `        <!-- SECTION 3 : Prescriptions -->
        <section class="glass-card p-4 flex flex-col gap-3 rounded-xl border bg-white/5 border-amber-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-amber-400 shrink-0">
            <span>💊</span> 3. Prescriptions LLM
          </h2>
          <div class="flex flex-col gap-3 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <div v-if="store.suggestions.length === 0" class="bg-black/20 rounded-lg border border-white/10 p-4 text-center text-gray-400 text-xs flex items-center justify-center">
              Aucune prescription pour l'instant.
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="s in store.suggestions" :key="\`\${s.strategie}-\${s.param_name}\`"
                class="bg-black/20 rounded-lg border border-white/10 p-3 flex flex-col gap-2"
              >
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="text-[9px] font-bold px-1.5 py-0.5 rounded" :class="badgeStrategie(s.strategie)">{{ s.strategie }}</span>
                    <span class="text-xs font-semibold">{{ s.param_name }}</span>
                  </div>
                  <div class="flex gap-1 shrink-0">
                    <button @click="appliquer(s)" :disabled="store.application" class="px-2 py-1 rounded bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 hover:bg-emerald-600/30 text-[10px] font-bold transition-colors">✓ Appliquer</button>
                    <button class="px-2 py-1 rounded bg-white/5 hover:bg-white/10 border border-white/10 text-[10px] text-gray-400 transition-colors">✗ Ignorer</button>
                  </div>
                </div>
                <div class="flex items-center gap-3 text-sm">
                  <span class="line-through text-gray-500">{{ s.valeur_actuelle }}</span> <span class="text-gray-600">→</span> 
                  <span class="text-emerald-400 font-bold">{{ s.valeur_suggeree }}</span>
                </div>
                <p class="text-[10px] text-gray-400 italic">"{{ s.justification }}"</p>
                <div class="flex items-center justify-between text-[9px] text-gray-500 border-t border-white/5 pt-1">
                  <span class="text-blue-400">Gains estimé : +{{ s.gain_winrate_estime.toFixed(1) }}% WR</span>
                  <span>Confiance : {{ (s.confiance * 100).toFixed(0) }}%</span>
                  <span>Basé sur {{ s.nb_samples_base }} trades</span>
                </div>
              </div>
            </div>

            <!-- Historique mini -->
            <div v-if="store.historique.length > 0" class="bg-black/20 rounded-lg border border-white/10 p-3 mt-auto shrink-0">
              <h3 class="text-[10px] font-semibold text-gray-400 mb-2 uppercase tracking-wide">Dernières applications</h3>
              <div class="space-y-1">
                <div v-for="h in store.historique.slice(0, 3)" :key="h.id" class="flex items-center justify-between text-[11px] py-1 border-b border-white/5 last:border-0 relative">
                  <span class="text-[9px] font-bold px-1 py-0.5 rounded w-12 text-center" :class="badgeStrategie(h.strategie)">{{ h.strategie.substring(0,3) }}</span>
                  <span class="text-gray-300 w-24 truncate px-2" :title="h.param_name">{{ h.param_name }}</span>
                  <span class="text-emerald-400 flex-1 text-center font-semibold">{{ h.valeur_apres }}</span>
                  <span class="text-gray-500 shrink-0 text-right">{{ new Date(h.appliquee_le * 1000).toLocaleDateString() }}</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- SECTION 4 : Seuils ML -->`
);

content = content.replace(
  /        <!-- SECTION 4 : Seuils ML -->[\s\S]*?<\/div>\n    <\/div>/,
  `        <!-- SECTION 4 : Seuils ML -->
        <section class="glass-card p-4 flex flex-col gap-3 rounded-xl border bg-white/5 border-red-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-red-400 shrink-0">
            <span>🤖</span> 4. Seuils de confiance ML
          </h2>
          <div class="flex flex-col gap-3 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <MlSeuilsPanel />
          </div>
        </section>

      </div>
    </div>`
);

fs.writeFileSync('frontend/src/views/MlInsightsView.vue', content);
