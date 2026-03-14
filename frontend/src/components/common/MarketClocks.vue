<template>
  <div class="glass-card p-5">
    <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Sessions de marché</h2>
    <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
      <div
        v-for="session in sessions"
        :key="session.nom"
        class="rounded-lg px-3 py-2.5 flex flex-col gap-1 transition-all"
        :class="session.classe"
      >
        <div class="flex items-center justify-between">
          <span class="text-[10px] uppercase tracking-wider font-medium" :class="session.labelCouleur">{{ session.nom }}</span>
          <span class="text-[10px] font-bold" :class="session.badgeCouleur">{{ session.statut }}</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-lg font-bold tabular-nums leading-none" :class="session.heureCouleur">{{ session.heureLocale }}</span>
          <div class="text-[8.5px] leading-tight text-gray-400">
            <div>Session heure locale : {{ session.plageLocale }}</div>
            <div>Heure Paris : {{ session.plageParis }}</div>
          </div>
        </div>
        <div class="text-[10px] font-medium" :class="session.countdownCouleur">{{ session.countdown }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface SessionDef {
  nom: string
  timezone: string
  ouvertureUtcH: number
  ouvertureUtcM: number
  fermetureUtcH: number
  fermetureUtcM: number
}

const SESSIONS: SessionDef[] = [
  { nom: 'Tokyo',     timezone: 'Asia/Tokyo',        ouvertureUtcH: 0,  ouvertureUtcM: 0,  fermetureUtcH: 9,  fermetureUtcM: 0 },
  { nom: 'Hong Kong', timezone: 'Asia/Hong_Kong',    ouvertureUtcH: 1,  ouvertureUtcM: 0,  fermetureUtcH: 9,  fermetureUtcM: 0 },
  { nom: 'Londres',   timezone: 'Europe/London',     ouvertureUtcH: 8,  ouvertureUtcM: 0,  fermetureUtcH: 17, fermetureUtcM: 0 },
  { nom: 'New York',  timezone: 'America/New_York',  ouvertureUtcH: 13, ouvertureUtcM: 30, fermetureUtcH: 20, fermetureUtcM: 0 },
]

const maintenant = ref(new Date())
let timer: ReturnType<typeof setInterval> | null = null

function padDeux(n: number): string {
  return String(n).padStart(2, '0')
}

function heureLocaleFormatee(date: Date, timezone: string): string {
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: timezone,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(date)
}

function convertirEnTz(utcH: number, utcM: number, timezone: string, dateRef: Date): string {
  const d = new Date(dateRef)
  d.setUTCHours(utcH, utcM, 0, 0)
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: timezone,
    hourCycle: 'h23',
    hour: '2-digit',
    minute: '2-digit',
  }).format(d)
}

function abreviationTz(timezone: string, dateRef: Date): string {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: timezone,
    timeZoneName: 'short',
  }).formatToParts(dateRef)
  return parts.find(p => p.type === 'timeZoneName')?.value ?? ''
}

function offsetTzStr(timezone: string, dateRef: Date): string {
  const parts = new Intl.DateTimeFormat('en-GB', {
    timeZone: timezone,
    timeZoneName: 'shortOffset',
  }).formatToParts(dateRef)
  const raw = parts.find(p => p.type === 'timeZoneName')?.value ?? 'GMT+0'
  return raw.replace('GMT', 'UTC')
}

function estWeekEnd(date: Date): boolean {
  const jour = date.getUTCDay()
  return jour === 0 || jour === 6
}

function secUtcDepuisMinuit(date: Date): number {
  return date.getUTCHours() * 3600 + date.getUTCMinutes() * 60 + date.getUTCSeconds()
}

function formaterDuree(sec: number): string {
  if (sec <= 0) return ''
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  const s = sec % 60
  if (h > 0) return `dans ${h}h ${padDeux(m)}m`
  if (m > 0) return `dans ${m}m ${padDeux(s)}s`
  return `dans ${s}s`
}

type Etat = 'weekend' | 'active' | 'bientot' | 'fermee'

function etatSession(s: SessionDef, now: Date): Etat {
  if (estWeekEnd(now)) return 'weekend'
  const nowSec = secUtcDepuisMinuit(now)
  const ouvSec = s.ouvertureUtcH * 3600 + s.ouvertureUtcM * 60
  const ferSec = s.fermetureUtcH * 3600 + s.fermetureUtcM * 60
  if (nowSec >= ouvSec && nowSec < ferSec) return 'active'
  if ((ouvSec - nowSec + 86400) % 86400 <= 1800) return 'bientot'
  return 'fermee'
}

function secAvantOuverture(s: SessionDef, now: Date): number {
  const ouvSec = s.ouvertureUtcH * 3600 + s.ouvertureUtcM * 60
  return (ouvSec - secUtcDepuisMinuit(now) + 86400) % 86400
}

function secAvantFermeture(s: SessionDef, now: Date): number {
  const ferSec = s.fermetureUtcH * 3600 + s.fermetureUtcM * 60
  return (ferSec - secUtcDepuisMinuit(now) + 86400) % 86400
}

const sessions = computed(() => {
  const now = maintenant.value
  return SESSIONS.map((s) => {
    const etat = etatSession(s, now)
    const heureLocale = heureLocaleFormatee(now, s.timezone)

    let statut: string, classe: string, labelCouleur: string
    let badgeCouleur: string, heureCouleur: string
    let countdownCouleur: string, countdown: string

    if (etat === 'weekend') {
      statut = '○ WEEK-END (ouverture lundi)'
      classe = 'bg-white/3 border border-white/5'
      labelCouleur = 'text-gray-600'
      badgeCouleur = 'text-gray-700'
      heureCouleur = 'text-gray-600'
      countdownCouleur = 'text-gray-700'
      countdown = ''
    } else if (etat === 'active') {
      statut = '● LIVE'
      classe = 'bg-emerald-500/10 border border-emerald-500/30'
      labelCouleur = 'text-emerald-300'
      badgeCouleur = 'text-emerald-400 animate-pulse'
      heureCouleur = 'text-white'
      countdownCouleur = 'text-emerald-500'
      countdown = `ferme ${formaterDuree(secAvantFermeture(s, now))}`
    } else if (etat === 'bientot') {
      statut = '◐ BIENTÔT'
      classe = 'bg-amber-500/10 border border-amber-500/30'
      labelCouleur = 'text-amber-300'
      badgeCouleur = 'text-amber-400 animate-pulse'
      heureCouleur = 'text-amber-200'
      countdownCouleur = 'text-amber-400 font-semibold'
      countdown = formaterDuree(secAvantOuverture(s, now))
    } else {
      statut = '○ FERMÉ'
      classe = 'bg-white/3 border border-white/8'
      labelCouleur = 'text-gray-500'
      badgeCouleur = 'text-gray-600'
      heureCouleur = 'text-gray-400'
      countdownCouleur = 'text-gray-600'
      countdown = formaterDuree(secAvantOuverture(s, now))
    }

    const ouvLocal = convertirEnTz(s.ouvertureUtcH, s.ouvertureUtcM, s.timezone, now)
    const ferLocal = convertirEnTz(s.fermetureUtcH, s.fermetureUtcM, s.timezone, now)
    const plageLocale = `${ouvLocal} – ${ferLocal} ${abreviationTz(s.timezone, now)}`

    const ouvParis = convertirEnTz(s.ouvertureUtcH, s.ouvertureUtcM, 'Europe/Paris', now)
    const ferParis = convertirEnTz(s.fermetureUtcH, s.fermetureUtcM, 'Europe/Paris', now)
    const plageParis = `${ouvParis} – ${ferParis} ${abreviationTz('Europe/Paris', now)} (${offsetTzStr('Europe/Paris', now)})`

    return { nom: s.nom, plageLocale, plageParis, statut, classe, labelCouleur, badgeCouleur, heureCouleur, countdownCouleur, countdown, heureLocale }
  })
})

onMounted(() => { timer = setInterval(() => { maintenant.value = new Date() }, 1000) })
onUnmounted(() => { if (timer !== null) clearInterval(timer) })
</script>
