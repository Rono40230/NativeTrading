<template>
  <div class="glass-card py-1.5 px-3 relative overflow-hidden shrink-0 flex items-center justify-center">
    <WorldMapBg />
    <div class="grid grid-cols-2 gap-2 lg:grid-cols-5 relative w-full">
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
  ouvertureUtcH: number; ouvertureUtcM: number
  fermetureUtcH: number; fermetureUtcM: number
}

const SESSIONS: SessionDef[] = [
  { nom: 'Hong Kong', timezone: 'Asia/Hong_Kong',    ouvertureUtcH: 1,  ouvertureUtcM: 0,  fermetureUtcH: 9,  fermetureUtcM: 0  },
  { nom: 'New York',  timezone: 'America/New_York',  ouvertureUtcH: 13, ouvertureUtcM: 30, fermetureUtcH: 20, fermetureUtcM: 0  },
  { nom: 'Londres',   timezone: 'Europe/London',     ouvertureUtcH: 8,  ouvertureUtcM: 0,  fermetureUtcH: 17, fermetureUtcM: 0  },
  { nom: 'Sydney',    timezone: 'Australia/Sydney',  ouvertureUtcH: 22, ouvertureUtcM: 0,  fermetureUtcH: 6,  fermetureUtcM: 0  },
  { nom: 'Tokyo',     timezone: 'Asia/Tokyo',        ouvertureUtcH: 0,  ouvertureUtcM: 0,  fermetureUtcH: 9,  fermetureUtcM: 0  },
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

function convertirEnTz(utcH: number, utcM: number, tz: string, ref: Date) {
  const d = new Date(ref); d.setUTCHours(utcH, utcM, 0, 0)
  return new Intl.DateTimeFormat('fr-FR', { timeZone: tz, hourCycle: 'h23', hour: '2-digit', minute: '2-digit' }).format(d)
}

function abrevTz(tz: string, date: Date) {
  return new Intl.DateTimeFormat('en-US', { timeZone: tz, timeZoneName: 'short' })
    .formatToParts(date).find(p => p.type === 'timeZoneName')?.value ?? ''
}

function estWeekEnd(d: Date) { const j = d.getUTCDay(); return j === 0 || j === 6 }

function secUtc(d: Date) { return d.getUTCHours() * 3600 + d.getUTCMinutes() * 60 + d.getUTCSeconds() }

function formatDuree(sec: number) {
  if (sec <= 0) return ''
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60
  if (h > 0) return `dans ${h}h ${pad(m)}m`
  if (m > 0) return `dans ${m}m ${pad(s)}s`
  return `dans ${s}s`
}

type Etat = 'weekend' | 'active' | 'bientot' | 'fermee'

function etatSession(s: SessionDef, now: Date): Etat {
  if (estWeekEnd(now)) return 'weekend'
  const cur = secUtc(now)
  const ouv = s.ouvertureUtcH * 3600 + s.ouvertureUtcM * 60
  const fer = s.fermetureUtcH * 3600 + s.fermetureUtcM * 60
  const actif = ouv > fer ? (cur >= ouv || cur < fer) : (cur >= ouv && cur < fer)
  if (actif) return 'active'
  if ((ouv - cur + 86400) % 86400 <= 1800) return 'bientot'
  return 'fermee'
}

function secAvantOuv(s: SessionDef, now: Date) {
  return (s.ouvertureUtcH * 3600 + s.ouvertureUtcM * 60 - secUtc(now) + 86400) % 86400
}
function secAvantFer(s: SessionDef, now: Date) {
  return (s.fermetureUtcH * 3600 + s.fermetureUtcM * 60 - secUtc(now) + 86400) % 86400
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

    const ouvLocal = convertirEnTz(s.ouvertureUtcH, s.ouvertureUtcM, s.timezone, now)
    const ferLocal = convertirEnTz(s.fermetureUtcH, s.fermetureUtcM, s.timezone, now)
    const plageLocale = `${ouvLocal} – ${ferLocal} ${abrevTz(s.timezone, now)}`

    const ouvParis = convertirEnTz(s.ouvertureUtcH, s.ouvertureUtcM, 'Europe/Paris', now)
    const ferParis = convertirEnTz(s.fermetureUtcH, s.fermetureUtcM, 'Europe/Paris', now)
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
