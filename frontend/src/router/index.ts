import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    // Dashboard
    { path: '/', component: () => import('../views/DashboardHome.vue') },

    // Stratégies
    { path: '/rockets',                    component: () => import('../views/RocketsView.vue') },
    { path: '/rockets/definition',         component: () => import('../views/RocketsDefinitionView.vue') },
    { path: '/rockets/scanner',           component: () => import('../views/RocketsScannerView.vue') },
    { path: '/smc/definition',             component: () => import('../views/SmcDefinitionView.vue') },
    { path: '/straddle/definition',        component: () => import('../views/StraddleDefinitionView.vue') },
    { path: '/smc',                 component: () => import('../views/SmcView.vue') },
    { path: '/smc/graphiques',      component: () => import('../views/ChartsView.vue') },
    { path: '/straddle',            component: () => import('../views/StraddleView.vue') },


    // Performance
    { path: '/heatmap',  component: () => import('../views/HeatmapView.vue') },

    // Rapport d'activité — centre d'analyse des stratégies (04/09).
    // ?strategie=SMC|straddle|rockets cible l'onglet (bloc dashboard).
    { path: '/analyses', component: () => import('../views/AnalysesView.vue') },

    // Outils IA — regroupés en une page à onglets (refonte navigation 01/09) ;
    // les anciens chemins redirigent vers l'onglet correspondant.
    { path: '/ia',            component: () => import('../views/IaView.vue') },
    { path: '/ia/chart',      redirect: { path: '/ia', query: { tab: 'chart' } } },
    { path: '/ia/coach',      redirect: { path: '/ia', query: { tab: 'coach' } } },
    { path: '/config/prompts', redirect: { path: '/ia', query: { tab: 'prompts' } } },
    { path: '/ml-insights', redirect: { path: '/ia', query: { tab: 'prompts' } } },

    // Presse
    { path: '/presse', component: () => import('../views/PresseView.vue') },

    // Données — pilotage du pipeline, risque par actif, connexions.
    // Paramètres — réglages des stratégies (bouton ⚙️ de chaque page
    // stratégie ; ?strategie=SMC n'en montre qu'une).
    { path: '/donnees',    component: () => import('../views/DonneesView.vue') },
    { path: '/parametres', component: () => import('../views/ParametresView.vue') },
    { path: '/systeme',  redirect: { path: '/donnees' } },
    { path: '/settings', redirect: { path: '/parametres' } },
    { path: '/data',     redirect: { path: '/donnees' } },

    // Historique global retiré (redondant avec les historiques par stratégie) —
    // l'ancien chemin renvoie au dashboard.
    { path: '/history',  redirect: { path: '/' } },
  ]
})

export default router
