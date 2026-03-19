<template>
  <div class="space-y-5">

    <!-- ── Swings H/L ─────────────────────────────────────────────────── -->
    <section>
      <div class="flex items-center justify-between mb-3">
        <span class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest">Swings H/L</span>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqSwingsActif" class="rounded accent-emerald-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div class="grid grid-cols-2 gap-3" :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqSwingsActif }">
        <div class="bg-white/5 rounded-lg p-2.5">
          <p class="text-[10px] text-slate-500 mb-1.5">High (BSL)</p>
          <div class="flex items-center gap-2">
            <input type="color" v-model="prefs.smcLiqCouleurBsl"
              class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
            <span class="text-[10px] text-slate-400 font-mono">{{ prefs.smcLiqCouleurBsl }}</span>
          </div>
        </div>
        <div class="bg-white/5 rounded-lg p-2.5">
          <p class="text-[10px] text-slate-500 mb-1.5">Low (SSL)</p>
          <div class="flex items-center gap-2">
            <input type="color" v-model="prefs.smcLiqCouleurSsl"
              class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
            <span class="text-[10px] text-slate-400 font-mono">{{ prefs.smcLiqCouleurSsl }}</span>
          </div>
        </div>
      </div>
    </section>

    <div class="border-t border-white/8" />

    <!-- ── Sessions H/L ───────────────────────────────────────────────── -->
    <section>
      <div class="flex items-center justify-between mb-3">
        <span class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest">Sessions H/L</span>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqSessionsActif" class="rounded accent-sky-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqSessionsActif }">
        <div v-for="sess in sessions" :key="sess.key"
          class="flex items-center justify-between bg-white/5 rounded-lg px-3 py-2">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" v-model="(prefs as any)[sess.checkKey]" class="rounded accent-slate-400" />
            <span class="text-xs text-slate-300">{{ sess.label }}</span>
          </label>
          <input type="color" v-model="(prefs as any)[sess.colorKey]"
            class="w-7 h-6 rounded cursor-pointer border border-white/15 bg-transparent" />
        </div>
      </div>
    </section>

    <div class="border-t border-white/8" />

    <!-- ── Range Asie ─────────────────────────────────────────────────── -->
    <section>
      <div class="flex items-center justify-between mb-3">
        <span class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest">Range Asie</span>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqAsieRangeActif" class="rounded accent-amber-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div class="space-y-2.5" :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqAsieRangeActif }">

        <!-- Couleur + Opacité -->
        <div class="grid grid-cols-2 gap-2">
          <div class="bg-white/5 rounded-lg px-3 py-2">
            <p class="text-[10px] text-slate-500 mb-1.5">Couleur</p>
            <div class="flex items-center gap-2">
              <input type="color" v-model="prefs.smcLiqAsieCouleur"
                class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
              <span class="text-[10px] text-slate-400 font-mono">{{ prefs.smcLiqAsieCouleur }}</span>
            </div>
          </div>
          <div class="bg-white/5 rounded-lg px-3 py-2">
            <p class="text-[10px] text-slate-500 mb-1.5">Opacité fond</p>
            <input type="number" min="0.05" max="0.5" step="0.05" v-model.number="prefs.smcLiqAsieOpacite"
              class="w-full text-center text-xs bg-[#0f1629] border border-white/10 rounded px-2 py-1 text-white focus:outline-none" />
          </div>
        </div>

        <!-- Sessions + Horaires -->
        <div class="grid grid-cols-2 gap-2">
          <div class="bg-white/5 rounded-lg px-3 py-2">
            <p class="text-[10px] text-slate-500 mb-1.5">Nb sessions</p>
            <input type="number" min="1" max="5" step="1" v-model.number="prefs.smcLiqAsieNbSessions"
              class="w-full text-center text-xs bg-[#0f1629] border border-white/10 rounded px-2 py-1 text-white focus:outline-none" />
          </div>
          <div class="bg-white/5 rounded-lg px-3 py-2">
            <p class="text-[10px] text-slate-500 mb-1.5">Horaires Paris</p>
            <div class="flex items-center gap-1 mb-1.5">
              <input type="number" min="0" max="23" step="1" v-model.number="prefs.smcLiqAsieHeureDebut"
                class="w-full text-center text-xs bg-[#0f1629] border border-white/10 rounded px-1 py-1 text-white focus:outline-none" />
              <span class="text-slate-500 text-[10px]">→</span>
              <input type="number" min="0" max="23" step="1" v-model.number="prefs.smcLiqAsieHeureFin"
                class="w-full text-center text-xs bg-[#0f1629] border border-white/10 rounded px-1 py-1 text-white focus:outline-none" />
            </div>
            <div class="flex items-center gap-1.5">
              <span class="text-[10px] text-slate-500">UTC+</span>
              <input type="number" min="0" max="12" step="1" v-model.number="prefs.smcLiqAsieOffsetUtc"
                class="w-12 text-center text-xs bg-[#0f1629] border border-white/10 rounded px-1 py-1 text-white focus:outline-none"
                title="1 = Paris hiver (CET), 2 = Paris été (CEST)" />
            </div>
          </div>
        </div>

        <!-- Déviations -->
        <div class="bg-white/5 rounded-lg px-3 py-2">
          <div class="flex items-center justify-between">
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" v-model="prefs.smcLiqAsieDeviationsActif" class="rounded accent-amber-500" />
              <span class="text-xs text-slate-300">Déviations</span>
            </label>
            <div class="flex items-center gap-2">
              <span class="text-[10px] text-slate-500">Nombre</span>
              <input type="number" min="1" max="5" step="1" v-model.number="prefs.smcLiqAsieDeviationsNb"
                :disabled="!prefs.smcLiqAsieDeviationsActif"
                class="w-12 text-center text-xs bg-[#0f1629] border border-white/10 rounded px-1 py-1 text-white focus:outline-none disabled:opacity-40" />
            </div>
          </div>
        </div>

      </div>
    </section>

  </div>
</template>

<script setup lang="ts">
import type { PrefsIndicateurs } from '@/stores/settings.store'

const prefs = defineModel<PrefsIndicateurs>({ required: true })

const sessions = [
  { key: 'asie', label: 'Asie (22h–07h UTC)', checkKey: 'smcLiqSessionAsie', colorKey: 'smcLiqCouleurAsie' },
]
</script>
