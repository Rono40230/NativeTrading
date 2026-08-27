<template>
  <div class="glass-card py-1.5 px-3 relative overflow-hidden shrink-0 flex items-center justify-center">
    <WorldMapBg />
    <div class="grid grid-cols-3 gap-2 lg:grid-cols-6 relative w-full mx-auto" style="max-width: 900px;">
      <div v-for="session in sessions" :key="session.nom" class="flex flex-col items-center gap-0">

        <!-- Nom de la place au-dessus -->
        <span class="text-[11px] font-extrabold uppercase tracking-wider mb-1" :class="session.labelCouleur">{{ session.nom }}</span>

        <!-- Cadran analogique SVG -->
        <svg viewBox="0 0 100 100" class="w-14 h-14 drop-shadow-xl mb-1">
          <!-- Fond -->
          <circle cx="50" cy="50" r="45" :fill="session.bgFill" />
          <!-- Anneau statut -->
          <circle cx="50" cy="50" r="45" fill="none" :stroke="session.ringColor"
            stroke-width="3" :class="session.ringAnim" />
          <!-- Ticks minutes -->
          <line v-for="t in TICKS_MIN" :key="`m${t.i}`"
            :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2"
            :stroke="session.tickColor" stroke-width="0.6" />
          <!-- Ticks heures (plus épais) -->
          <line v-for="t in TICKS_HR" :key="`h${t.i}`"
            :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2"
            :stroke="session.tickColor" stroke-width="2" stroke-linecap="round" />
          <!-- Aiguille heures -->
          <line x1="50" y1="50" :x2="session.hrX" :y2="session.hrY"
            :stroke="session.handColor" stroke-width="4.5" stroke-linecap="round" />
          <!-- Aiguille minutes -->
          <line x1="50" y1="50" :x2="session.minX" :y2="session.minY"
            :stroke="session.handColor" stroke-width="3" stroke-linecap="round" />
          <!-- Aiguille secondes (avec queue) -->
          <line :x1="session.secTailX" :y1="session.secTailY"
            :x2="session.secX" :y2="session.secY"
            :stroke="session.secColor" stroke-width="1.5" stroke-linecap="round" />
          <!-- Centre -->
          <circle cx="50" cy="50" r="3.5" :fill="session.secColor" />
          <circle cx="50" cy="50" r="1.5" fill="#0b0f28" />
        </svg>

        <!-- Infos sous le cadran -->
        <div class="flex flex-col items-center gap-0 text-center w-full">
          <div class="flex items-center justify-center gap-1 flex-wrap">
            <span class="text-[8px] font-bold" :class="session.badgeCouleur">{{ session.statutCourt }}</span>
            <span v-if="session.countdown" class="text-[8px] font-semibold" :class="session.countdownCouleur">{{ session.countdown }}</span>
          </div>
          <span class="text-sm font-mono font-bold tabular-nums leading-none" :class="session.heureCouleur">{{ session.heureLocale }}</span>
          <div class="text-[8px] leading-tight flex items-center justify-center gap-1 flex-wrap">
            <span class="text-gray-600">{{ session.plageLocale }}</span>
            <span class="text-blue-300/60">🇫🇷 {{ session.plageParis }}</span>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import WorldMapBg from './WorldMapBg.vue'

interface SessionDef {
  nom: string; timezone: string
  // Horaires en **heure locale de la place** (wall-clock, inchangés par le DST
  // de la place). L'offset UTC est calculé dynamiquement via Intl.DateTimeFormat
  // avec le timeZone de la ville — fini les bornes UTC figées incorrectes en
  // hiver/été.
  ouvertureLocaleH: number; ouvertureLocaleM: number
  fermetureLocaleH: number; fermetureLocaleM: number
}

const SESSIONS: SessionDef[] = [
  { nom: 'Hong Kong', timezone: 'Asia/Hong_Kong',    ouvertureLocaleH: 9,  ouvertureLocaleM: 0,  fermetureLocaleH: 17, fermetureLocaleM: 0  },
  { nom: 'New York',  timezone: 'America/New_York',  ouvertureLocaleH: 9,  ouvertureLocaleM: 30, fermetureLocaleH: 16, fermetureLocaleM: 0  },
  { nom: 'Londres',   timezone: 'Europe/London',     ouvertureLocaleH: 8,  ouvertureLocaleM: 0,  fermetureLocaleH: 17, fermetureLocaleM: 0  },
  { nom: 'Paris',    timezone: 'Europe/Paris',      ouvertureLocaleH: 9,  ouvertureLocaleM: 0,  fermetureLocaleH: 17, fermetureLocaleM: 30 },
  { nom: 'Sydney',    timezone: 'Australia/Sydney',  ouvertureLocaleH: 8,  ouvertureLocaleM: 0,  fermetureLocaleH: 16, fermetureLocaleM: 0  },
  { nom: 'Tokyo',     timezone: 'Asia/Tokyo',        ouvertureLocaleH: 9,  ouvertureLocaleM: 0,  fermetureLocaleH: 18, fermetureLocaleM: 0  },
]

// Ticks pré-calculés
const TICKS_MIN = Array.from({ length: 60 }, (_, i) => {
  const a = (i / 60) * 2 * Math.PI
  return { i, x1: 50 + 43 * Math.sin(a), y1: 50 - 43 * Math.cos(a), x2: 50 + 45 * Math.sin(a), y2: 50 - 45 * Math.cos(a) }
})
const TICKS_HR = Array.from({ length: 12 }, (_, i) => {
  const a = (i / 12) * 2 * Math.PI
  return { i, x1: 50 + 37 * Math.sin(a), y1: 50 - 37 * Math.cos(a), x2: 50 + 45 * Math.sin(a), y2: 50 - 45 * Math.cos(a) }
})

const maintenant = ref(new Date())
let timer: ReturnType<typeof setInterval> | null = null

function pad(n: number) { return String(n).padStart(2, '0') }

function getTimeParts(timezone: string, date: Date) {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: timezone, hour: 'numeric', minute: 'numeric', second: 'numeric', hour12: false,
  }).formatToParts(date)
  const g = (t: string) => parseInt(parts.find(p => p.type === t)?.value ?? '0')
  return { h: g('hour') % 24, m: g('minute'), s: g('second') }
}

function handXY(angleDeg: number, len: number) {
  const r = angleDeg * Math.PI / 180
  return { x: 50 + len * Math.sin(r), y: 50 - len * Math.cos(r) }
}

function heureLocaleFormatee(tz: string, date: Date) {
  return new Intl.DateTimeFormat('fr-FR', { timeZone: tz, hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }).format(date)
}

function abrevTz(tz: string, date: Date) {
  return new Intl.DateTimeFormat('en-US', { timeZone: tz, timeZoneName: 'short' })
    .formatToParts(date).find(p => p.type === 'timeZoneName')?.value ?? ''
}

/** Secondes écoulées dans la journée locale de la timezone `tz`. */
function secLocale(tz: string, date: Date) {
  const { h, m, s } = getTimeParts(tz, date)
  return h * 3600 + m * 60 + s
}
function ouvLocaleSec(s: SessionDef) { return s.ouvertureLocaleH * 3600 + s.ouvertureLocaleM * 60 }
function ferLocaleSec(s: SessionDef) { return s.fermetureLocaleH * 3600 + s.fermetureLocaleM * 60 }

/** Jour de semaine (0=Dim..6=Sam) dans la timezone donnée. */
function jourSemaineTz(tz: string, date: Date): number {
  const wd = new Intl.DateTimeFormat('en-US', { timeZone: tz, weekday: 'short' }).format(date)
  const map: Record<string, number> = { Sun: 0, Mon: 1, Tue: 2, Wed: 3, Thu: 4, Fri: 5, Sat: 6 }
  return map[wd] ?? 0
}
function estWeekEndTz(tz: string, date: Date) { const j = jourSemaineTz(tz, date); return j === 0 || j === 6 }

/**
 * Convertit une heure locale (HH:MM) de `tzSource` vers l'heure de `tzCible`,
 * à la date `ref`. L'instant cible est obtenu en décalant `ref` du delta entre
 * l'heure locale voulue et l'heure locale courante — le DST du jour est ainsi
 * pris en compte via les parts Intl.
 */
function convertirLocaleVersTz(locH: number, locM: number, tzSource: string, tzCible: string, ref: Date) {
  const src = getTimeParts(tzSource, ref)
  const deltaMin = (locH * 60 + locM) - (src.h * 60 + src.m)
  const instant = new Date(ref.getTime() + deltaMin * 60000)
  return new Intl.DateTimeFormat('fr-FR', { timeZone: tzCible, hourCycle: 'h23', hour: '2-digit', minute: '2-digit' }).format(instant)
}

function formatDuree(sec: number) {
  if (sec <= 0) return ''
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60
  if (h > 0) return `dans ${h}h ${pad(m)}m`
  if (m > 0) return `dans ${m}m ${pad(s)}s`
  return `dans ${s}s`
}

type Etat = 'weekend' | 'active' | 'bientot' | 'fermee'

function etatSession(s: SessionDef, now: Date): Etat {
  if (estWeekEndTz(s.timezone, now)) return 'weekend'
  const cur = secLocale(s.timezone, now)
  const ouv = ouvLocaleSec(s)
  const fer = ferLocaleSec(s)
  const actif = ouv > fer ? (cur >= ouv || cur < fer) : (cur >= ouv && cur < fer)
  if (actif) return 'active'
  if ((ouv - cur + 86400) % 86400 <= 1800) return 'bientot'
  return 'fermee'
}

function secAvantOuv(s: SessionDef, now: Date) {
  return (ouvLocaleSec(s) - secLocale(s.timezone, now) + 86400) % 86400
}
function secAvantFer(s: SessionDef, now: Date) {
  return (ferLocaleSec(s) - secLocale(s.timezone, now) + 86400) % 86400
}

const sessions = computed(() => {
  const now = maintenant.value
  return SESSIONS.map(s => {
    const etat = etatSession(s, now)
    const { h, m: mm, s: ss } = getTimeParts(s.timezone, now)
    const heureLocale = heureLocaleFormatee(s.timezone, now)

    const hrAngle  = ((h % 12) / 12 + mm / 720) * 360
    const minAngle = (mm / 60 + ss / 3600) * 360
    const secAngle = (ss / 60) * 360

    const hr = handXY(hrAngle, 23)
    const min = handXY(minAngle, 32)
    const sec = handXY(secAngle, 38)
    const secTail = handXY(secAngle + 180, 9)

    // Horaires locaux de la place (wall-clock, fixes) + équivalent Paris.
    const ouvLocal = `${pad(s.ouvertureLocaleH)}:${pad(s.ouvertureLocaleM)}`
    const ferLocal = `${pad(s.fermetureLocaleH)}:${pad(s.fermetureLocaleM)}`
    const plageLocale = `${ouvLocal} – ${ferLocal} ${abrevTz(s.timezone, now)}`

    const ouvParis = convertirLocaleVersTz(s.ouvertureLocaleH, s.ouvertureLocaleM, s.timezone, 'Europe/Paris', now)
    const ferParis = convertirLocaleVersTz(s.fermetureLocaleH, s.fermetureLocaleM, s.timezone, 'Europe/Paris', now)
    const plageParis = `${ouvParis} – ${ferParis} Paris`

    let ringColor: string, bgFill: string, handColor: string, secColor: string
    let tickColor: string, ringAnim: string
    let labelCouleur: string, badgeCouleur: string, heureCouleur: string
    let countdownCouleur: string, countdown: string, statutCourt: string

    if (etat === 'active') {
      ringColor = '#10b981'; bgFill = 'rgba(16,185,129,0.08)'
      handColor = '#ffffff'; secColor = '#10b981'; tickColor = 'rgba(255,255,255,0.3)'
      ringAnim = 'ring-live'; labelCouleur = 'text-emerald-300'
      badgeCouleur = 'text-emerald-400'; heureCouleur = 'text-white'
      countdownCouleur = 'text-emerald-500'; statutCourt = '● LIVE'
      countdown = `ferme ${formatDuree(secAvantFer(s, now))}`
    } else if (etat === 'bientot') {
      ringColor = '#f59e0b'; bgFill = 'rgba(245,158,11,0.08)'
      handColor = '#fcd34d'; secColor = '#f59e0b'; tickColor = 'rgba(255,255,255,0.2)'
      ringAnim = 'ring-soon'; labelCouleur = 'text-amber-300'
      badgeCouleur = 'text-amber-400'; heureCouleur = 'text-amber-200'
      countdownCouleur = 'text-amber-400'; statutCourt = '◐ BIENTÔT'
      countdown = formatDuree(secAvantOuv(s, now))
    } else if (etat === 'weekend') {
      ringColor = 'rgba(255,255,255,0.06)'; bgFill = 'rgba(255,255,255,0.02)'
      handColor = '#374151'; secColor = '#374151'; tickColor = 'rgba(255,255,255,0.07)'
      ringAnim = ''; labelCouleur = 'text-gray-600'
      badgeCouleur = 'text-gray-700'; heureCouleur = 'text-gray-600'
      countdownCouleur = 'text-gray-700'; statutCourt = '○ W-E'
      countdown = ''
    } else {
      ringColor = 'rgba(255,255,255,0.14)'; bgFill = 'rgba(255,255,255,0.03)'
      handColor = '#6b7280'; secColor = '#4b5563'; tickColor = 'rgba(255,255,255,0.15)'
      ringAnim = ''; labelCouleur = 'text-gray-500'
      badgeCouleur = 'text-gray-600'; heureCouleur = 'text-gray-400'
      countdownCouleur = 'text-gray-600'; statutCourt = '○ FERMÉ'
      const duree = formatDuree(secAvantOuv(s, now))
      countdown = duree ? `ouvre ${duree}` : ''
    }

    return {
      nom: s.nom, heureLocale, plageLocale, plageParis, statutCourt, countdown,
      labelCouleur, badgeCouleur, heureCouleur, countdownCouleur,
      hrX: hr.x, hrY: hr.y, minX: min.x, minY: min.y,
      secX: sec.x, secY: sec.y, secTailX: secTail.x, secTailY: secTail.y,
      ringColor, bgFill, handColor, secColor, tickColor, ringAnim,
    }
  })
})

onMounted(() => { timer = setInterval(() => { maintenant.value = new Date() }, 1000) })
onUnmounted(() => { if (timer !== null) clearInterval(timer) })
</script>

<style scoped>
.ring-live  { animation: ring-pulse 2s ease-in-out infinite; }
.ring-soon  { animation: ring-pulse 1.2s ease-in-out infinite; }
@keyframes ring-pulse {
  0%, 100% { opacity: 0.9; }
  50%       { opacity: 0.35; }
}
</style>
