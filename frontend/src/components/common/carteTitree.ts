import { defineComponent, h } from 'vue'

/// Mini-composants partagés des pages Caractéristiques : carte titrée
/// (bleue SMC) + vignette de valeur. Extraits de SmcDefinitionView pour
/// respecter la limite de 600 lignes par fichier (pre-commit).
export const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', {
        class: 'text-xs font-semibold text-blue-400 uppercase tracking-widest mb-2.5',
        innerHTML: props.titre,
      }),
      h('div', {
        class: 'text-white text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0',
      }, slots.default?.()),
    ])
  },
})

export const Valeur = defineComponent({
  props: {
    etiquette: { type: String, required: true },
    valeur: { type: String, required: true },
  },
  setup: (p: { etiquette: string; valeur: string }) => () =>
    h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-4 py-3' }, [
      h('div', { class: 'text-[10px] text-white uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
