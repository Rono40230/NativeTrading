const fs = require('fs');
let content = fs.readFileSync('frontend/src/components/common/MlSeuilsPanel.vue', 'utf8');

// Supprimer le bouton "Enregistrer" et remplacer l'endroit
content = content.replace(
  /      <!-- Bouton enregistrer -->[\s\S]*?<\/div>\n\n    <\/div>/,
  `      <!-- Statut enregistrement auto -->
      <div class="pt-1 flex items-center justify-end h-4">
        <transition name="fade">
          <span v-if="message" class="text-[10px] font-bold" :class="messageOk ? 'text-emerald-400' : 'text-red-400'">
            {{ sauvegarde ? '⏳ Enregistrement...' : message }}
          </span>
        </transition>
      </div>

    </div>`
);

// Ajouter `@change="enregistrer"` aux inputs range (il y en a 3)
content = content.replace(/class="w-full accent-emerald-500 h-1 cursor-pointer"\n        \/>/g, `class="w-full accent-emerald-500 h-1 cursor-pointer"\n          @change="enregistrer"\n        />`);
content = content.replace(/class="w-full accent-blue-500 h-1 cursor-pointer"\n        \/>/g, `class="w-full accent-blue-500 h-1 cursor-pointer"\n          @change="enregistrer"\n        />`);
content = content.replace(/class="w-full accent-violet-500 h-1 cursor-pointer"\n        \/>/g, `class="w-full accent-violet-500 h-1 cursor-pointer"\n          @change="enregistrer"\n        />`);

// Refactor script pour exposer chargerSeuils
content = content.replace(
  /onMounted\(async \(\) => {[\s\S]*?}\)/,
  `async function chargerSeuils() {
  chargement.value = true
  const [r, s, m] = await Promise.all([
    chargerSeuil('seuil_confiance_rockets'),
    chargerSeuil('seuil_confiance_straddle'),
    chargerSeuil('seuil_confiance_smc'),
  ])
  if (r !== null) seuils.value.rockets = r
  if (s !== null) seuils.value.straddle = s
  if (m !== null) seuils.value.smc = m
  chargement.value = false
}

onMounted(() => {
  chargerSeuils()
})

defineExpose({ chargerSeuils })`
);

fs.writeFileSync('frontend/src/components/common/MlSeuilsPanel.vue', content);
