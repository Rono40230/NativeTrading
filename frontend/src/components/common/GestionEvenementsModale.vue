<template>
  <!-- ⚙ GÉRER LES CRÉNEAUX FIXES (owner 15/10) — niveau 2 de la pile
       (🕐 Choix des Assets & créneaux → ici → édition). Les créneaux
       structurels vivent en base depuis le 15/10 : créer / modifier /
       mettre en pause / supprimer, la matrice suit au refresh. Les 9 types
       📅 (annonces réelles) restent en code — rappelés en pied, non
       éditables (doctrine 14/10). -->
  <ModaleCadre titre="⚙ Gérer les créneaux fixes" large @fermer="$emit('fermer')">

    <div v-if="chargement" class="text-[11px] text-white/70 py-3 text-center">Chargement…</div>

    <template v-else>
      <!-- La liste : une ligne par créneau fixe -->
      <div class="overflow-auto rounded-lg border border-white/10 max-h-[46vh]">
        <table class="text-[11px] border-collapse min-w-full">
          <thead>
            <tr class="bg-black/30 text-white/60 font-semibold uppercase tracking-wide">
              <th class="px-2 py-1.5 text-left">Créneau</th>
              <th class="px-2 py-1.5 text-left">Horaire</th>
              <th class="px-2 py-1.5 text-left">Jours</th>
              <th class="px-2 py-1.5 text-center">État</th>
              <th class="px-2 py-1.5 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="ev in structurels" :key="ev.ident" class="border-t border-white/5"
                :class="ev.actif ? '' : 'opacity-50'">
              <td class="px-2 py-1.5">
                <div class="flex flex-col leading-tight">
                  <span class="font-semibold text-white">📯 {{ ev.nom }}</span>
                  <span class="text-[10px] text-white/50 font-mono">{{ ev.ident }}</span>
                </div>
              </td>
              <td class="px-2 py-1.5">
                <div class="flex flex-col leading-tight">
                  <span class="font-mono text-white/85">
                    {{ String(ev.heure).padStart(2, '0') }}:{{ String(ev.minute).padStart(2, '0') }}
                    <span class="text-white/50">· {{ TZ_COURTS[ev.tz] ?? ev.tz }}</span>
                  </span>
                  <span class="text-[10px] text-white/50 font-mono">
                    {{ ev.prochaine_heure_paris ? `→ ${ev.prochaine_heure_paris} Paris` : '—' }}
                  </span>
                </div>
              </td>
              <td class="px-2 py-1.5">
                <div class="flex gap-0.5">
                  <span v-for="j in 7" :key="j" class="w-4 text-center font-bold"
                        :class="ev.jours.includes(j) ? 'text-emerald-300' : 'text-white/15'">
                    {{ JOURS_COURTS[j - 1] }}
                  </span>
                </div>
              </td>
              <td class="px-2 py-1.5 text-center">
                <span class="text-[10px] font-bold px-1.5 py-0.5 rounded-full border"
                      :class="ev.actif
                        ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
                        : 'border-amber-500/40 bg-amber-500/10 text-amber-300'">
                  {{ ev.actif ? 'Actif' : 'En pause' }}
                </span>
              </td>
              <td class="px-2 py-1.5">
                <div class="flex items-center justify-end gap-1">
                  <button class="text-[10px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-white/20 text-white transition"
                          title="Modifier ce créneau" @click="ouvrirEdition(ev)">✏️ Modifier</button>
                  <button class="text-[10px] px-1.5 py-0.5 rounded transition"
                          :class="ev.actif
                            ? 'bg-amber-500/15 text-amber-300 hover:bg-amber-500/25'
                            : 'bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'"
                          :title="ev.actif
                            ? 'Mettre en pause : le moteur ne tire plus, les compteurs sont conservés'
                            : 'Réactiver ce créneau'"
                          @click="basculerActif(ev)">{{ ev.actif ? '⏸' : '▶' }}</button>
                  <button class="text-[10px] px-1.5 py-0.5 rounded bg-red-900/40 text-red-300 hover:bg-red-900/60 transition"
                          title="Supprimer le créneau ET ses cases de matrice"
                          @click="suppressionCible = ev">🗑</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Pied : ajout + rappel des types 📅 non éditables -->
      <div class="flex items-center gap-2 mt-2.5 flex-wrap">
        <button class="text-[11px] px-2.5 py-1 rounded-lg bg-emerald-600/20 text-emerald-300 font-semibold hover:bg-emerald-600/30 transition"
                @click="ouvrirEdition(null)">＋ Ajouter un créneau</button>
        <p class="text-[10px] text-white/50 leading-snug ml-auto max-w-[380px]">
          Un créneau créé arrive <strong>désarmé partout</strong> — armez ses cases dans la matrice.
          Les types 📅 ({{ typesCalendrier.map(t => t.nom.split(' ')[0]).join(', ') }}…) vivent
          des annonces réelles : non éditables.
        </p>
      </div>
    </template>

    <!-- Confirmation de suppression (design de l'app — plus de dialog natif) -->
    <ModaleConfirmation
      :ouverte="suppressionCible !== null"
      titre="Supprimer ce créneau ?"
      :sous-titre="suppressionCible ? `📯 ${suppressionCible.nom}` : ''"
      :lignes="suppressionCible
        ? [`${suppressionCible.ident} · ${String(suppressionCible.heure).padStart(2, '0')}:${String(suppressionCible.minute).padStart(2, '0')} ${TZ_COURTS[suppressionCible.tz] ?? suppressionCible.tz}`]
        : []"
      avertissement="Ses cases de matrice (compteurs et verdicts compris) seront supprimées. Le vécu garde ses trades, mais la ligne disparaît des stats par source."
      label-confirmer="Supprimer"
      variant="danger"
      icone="🗑"
      @confirmer="confirmerSuppression"
      @annuler="suppressionCible = null"
    />

    <!-- NIVEAU 3 : création / modification d'un créneau -->
    <ModaleCadre v-if="editionOuverte" :titre="edite ? '✏️ Modifier le créneau' : '＋ Nouveau créneau'" @fermer="editionOuverte = false">
      <div class="flex flex-col gap-2.5">
        <!-- Nom -->
        <label class="flex flex-col gap-1">
          <span class="text-[10px] uppercase tracking-wide text-white/60 font-semibold">Nom du créneau</span>
          <input v-model="form.nom" type="text" maxlength="80" placeholder="ex. Réouverture or (pause Axi)"
                 class="bg-white/10 rounded-lg px-2 py-1.5 text-sm text-white outline-none border border-white/10 focus:border-sky-400/50">
        </label>

        <!-- Mode horaire : Paris fixe ou ancré sur une place -->
        <div class="flex flex-col gap-1">
          <span class="text-[10px] uppercase tracking-wide text-white/60 font-semibold">Horaire de tirage</span>
          <div class="flex gap-1.5">
            <button class="text-[11px] px-2 py-1 rounded-lg border transition"
                    :class="form.tz === 'Europe/Paris'
                      ? 'bg-sky-500/20 border-sky-400/50 text-sky-200' : 'bg-white/[0.03] border-white/10 text-white/60 hover:bg-white/10'"
                    @click="form.tz = 'Europe/Paris'">Heure de Paris fixe</button>
            <select v-model="form.tz"
                    class="text-[11px] bg-white/10 rounded-lg border border-white/10 px-2 py-1 text-white outline-none">
              <option v-for="tz in TZ_CHOIX" :key="tz" :value="tz" class="bg-slate-900">
                Ancré : {{ TZ_COURTS[tz] ?? tz }} (heure locale)
              </option>
            </select>
          </div>
          <p class="text-[10px] text-white/45 leading-snug">
            {{ form.tz === 'Europe/Paris'
              ? 'L\'heure saisie est l\'heure de Paris TOUTE l\'année.'
              : 'L\'heure saisie est l\'heure LOCALE de la place — les bascules été/hiver suivent seules (ex. Tokyo 9h = 2h Paris en été, 1h en hiver).' }}
          </p>
          <div class="flex items-center gap-1.5">
            <input v-model.number="form.heure" type="number" min="0" max="23"
                   class="w-14 bg-white/10 rounded-lg px-2 py-1 text-sm font-mono text-white outline-none border border-white/10">
            <span class="text-white/60 text-sm">h</span>
            <input v-model.number="form.minute" type="number" min="0" max="59"
                   class="w-14 bg-white/10 rounded-lg px-2 py-1 text-sm font-mono text-white outline-none border border-white/10">
            <span class="text-white/60 text-sm">min — {{ TZ_COURTS[form.tz] ?? form.tz }}</span>
          </div>
        </div>

        <!-- Jours -->
        <div class="flex flex-col gap-1">
          <span class="text-[10px] uppercase tracking-wide text-white/60 font-semibold">Jours de tirage</span>
          <div class="flex gap-1">
            <button v-for="j in 7" :key="j"
                    class="w-8 h-7 rounded-lg border text-[11px] font-bold transition"
                    :class="form.jours.includes(j)
                      ? 'bg-emerald-500/20 border-emerald-400/50 text-emerald-200'
                      : 'bg-white/[0.03] border-white/10 text-white/40 hover:bg-white/10'"
                    @click="basculerJour(j)">{{ JOURS_COURTS[j - 1] }}</button>
          </div>
          <p class="text-[10px] text-white/45">Décocher samedi/dimanche exclut le week-end (défaut lun-ven).</p>
        </div>

        <!-- Description (facultatif) -->
        <label class="flex flex-col gap-1">
          <span class="text-[10px] uppercase tracking-wide text-white/60 font-semibold">Description (facultatif)</span>
          <textarea v-model="form.detail" rows="2" maxlength="300"
                    class="bg-white/10 rounded-lg px-2 py-1.5 text-xs text-white outline-none border border-white/10 focus:border-sky-400/50 resize-none"></textarea>
        </label>

        <!-- Actif (édition seule) -->
        <label v-if="edite" class="flex items-center gap-2">
          <input v-model="form.actif" type="checkbox" class="accent-emerald-500">
          <span class="text-[11px] text-white/80">Créneau actif (décocher = pause : le moteur ne tire plus, compteurs conservés)</span>
        </label>

        <!-- ⚠ Changement d'horaire : le warning + raz (owner 15/10 point 5) -->
        <div v-if="edite && horaireChange" class="rounded-lg border border-amber-500/40 bg-amber-500/10 p-2 flex flex-col gap-1">
          <p class="text-[11px] text-amber-200 leading-snug">
            ⚠ L'horaire de tirage change : les compteurs existants vont mélanger
            les passes d'avant et d'après le changement.
          </p>
          <label class="flex items-center gap-2">
            <input v-model="form.raz_compteurs" type="checkbox" class="accent-amber-500">
            <span class="text-[11px] text-amber-100">Remettre les compteurs à zéro (nouveau test)</span>
          </label>
        </div>

        <p v-if="erreur" class="text-[11px] text-red-300">{{ erreur }}</p>

        <div class="flex items-center justify-end gap-2 pt-1">
          <button class="text-[11px] px-2.5 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white transition"
                  :disabled="envoi" @click="editionOuverte = false">Annuler</button>
          <button class="text-[11px] px-2.5 py-1.5 rounded-lg font-semibold transition disabled:opacity-40"
                  :class="valide ? 'bg-emerald-600/30 text-emerald-200 hover:bg-emerald-600/40' : 'bg-white/10 text-white/40'"
                  :disabled="!valide || envoi" @click="valider">{{ envoi ? '⏳…' : edite ? 'Valider les modifications' : 'Créer le créneau' }}</button>
        </div>
      </div>
    </ModaleCadre>
  </ModaleCadre>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'
import ModaleCadre from './ModaleCadre.vue'
import ModaleConfirmation from './ModaleConfirmation.vue'
import { useAlerteStore } from '@/stores/alerte.store'

/// Le gestionnaire vit au-dessus de la modale 🕐 ; chaque changement
/// remonte par l'événement « change » → le parent recharge la matrice.
const emit = defineEmits<{ (e: 'fermer'): void; (e: 'change'): void }>()
const alerteStore = useAlerteStore()

const JOURS_COURTS = ['L', 'M', 'M', 'J', 'V', 'S', 'D']
const TZ_COURTS: Record<string, string> = {
  'Europe/Paris': 'Paris',
  'Europe/London': 'Londres',
  'America/New_York': 'New York',
  'Asia/Tokyo': 'Tokyo',
  'Australia/Sydney': 'Sydney',
  'UTC': 'UTC',
}
const TZ_CHOIX = Object.keys(TZ_COURTS)

interface CreneauFixe {
  ident: string
  nom: string
  detail: string
  tz: string
  heure: number
  minute: number
  jours: number[]
  actif: boolean
  prochaine_heure_paris: string | null
}

const chargement = ref(true)
const structurels = ref<CreneauFixe[]>([])
const typesCalendrier = ref<{ ident: string; nom: string }[]>([])

async function charger() {
  chargement.value = true
  try {
    const r = await http.get<{ structurels: CreneauFixe[]; types_calendrier: { ident: string; nom: string }[] }>(
      '/api/evenements/catalogue')
    structurels.value = r.data.structurels
    typesCalendrier.value = r.data.types_calendrier
  } catch (e) {
    alerteStore.afficherErreur(`Catalogue : ${(e as Error).message}`)
  }
  chargement.value = false
}

// ── Édition (niveau 3) ───────────────────────────────────────────────────────
const editionOuverte = ref(false)
const edite = ref<CreneauFixe | null>(null)
const envoi = ref(false)
const erreur = ref('')

const form = ref({
  nom: '',
  detail: '',
  tz: 'Europe/Paris',
  heure: 9,
  minute: 0,
  jours: [1, 2, 3, 4, 5] as number[],
  actif: true,
  raz_compteurs: true,
})

function ouvrirEdition(ev: CreneauFixe | null) {
  edite.value = ev
  erreur.value = ''
  form.value = ev
    ? { nom: ev.nom, detail: ev.detail, tz: ev.tz, heure: ev.heure, minute: ev.minute,
        jours: [...ev.jours], actif: ev.actif, raz_compteurs: true }
    : { nom: '', detail: '', tz: 'Europe/Paris', heure: 9, minute: 0,
        jours: [1, 2, 3, 4, 5], actif: true, raz_compteurs: true }
  editionOuverte.value = true
}

function basculerJour(j: number) {
  const i = form.value.jours.indexOf(j)
  if (i >= 0) form.value.jours.splice(i, 1)
  else form.value.jours.push(j)
}

const valide = computed(() =>
  form.value.nom.trim().length >= 2 && form.value.jours.length > 0)

/// L'horaire de TIRAGE change-t-il ? (fuseau, heure, minute ou jours)
const horaireChange = computed(() => {
  const ev = edite.value
  if (!ev) return false
  return form.value.tz !== ev.tz
    || form.value.heure !== ev.heure
    || form.value.minute !== ev.minute
    || [...form.value.jours].sort().join() !== [...ev.jours].sort().join()
})

async function valider() {
  envoi.value = true
  erreur.value = ''
  const corps = {
    nom: form.value.nom.trim(),
    detail: form.value.detail.trim() || undefined,
    tz: form.value.tz,
    heure: form.value.heure,
    minute: form.value.minute,
    jours: [...form.value.jours].sort(),
  }
  try {
    if (edite.value) {
      await http.put(`/api/evenements/catalogue/${edite.value.ident}`, {
        ...corps,
        actif: form.value.actif,
        raz_compteurs: horaireChange.value ? form.value.raz_compteurs : undefined,
      })
    } else {
      await http.post('/api/evenements/catalogue', corps)
    }
    editionOuverte.value = false
    await charger()
    emit('change')
  } catch (e) {
    erreur.value = (e as Error).message
  }
  envoi.value = false
}

async function basculerActif(ev: CreneauFixe) {
  try {
    await http.put(`/api/evenements/catalogue/${ev.ident}`, { actif: !ev.actif })
    await charger()
    emit('change')
  } catch (e) {
    alerteStore.afficherErreur(`Pause/activation : ${(e as Error).message}`)
  }
}

const suppressionCible = ref<CreneauFixe | null>(null)

async function confirmerSuppression() {
  const ev = suppressionCible.value
  if (!ev) return
  suppressionCible.value = null
  try {
    await http.delete(`/api/evenements/catalogue/${ev.ident}`)
    await charger()
    emit('change')
  } catch (e) {
    alerteStore.afficherErreur(`Suppression : ${(e as Error).message}`)
  }
}

onMounted(charger)
</script>
