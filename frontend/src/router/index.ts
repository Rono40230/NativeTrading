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
    { path: '/smc/scanner',         component: () => import('../views/SmcScannerView.vue') },
    { path: '/smc/graphiques',      component: () => import('../views/ChartsView.vue') },
    { path: '/straddle',            component: () => import('../views/StraddleView.vue') },
    { path: '/kdj',                component: () => import('../views/KdjView.vue') },
    { path: '/kdj/definition',     component: () => import('../views/KdjDefinitionView.vue') },
    { path: '/kdj/scanner',        component: () => import('../views/KdjScannerView.vue') },


    // Performance

    // Laboratoire de simulation — re-jeu paramétrique à la demande (15/09).
    // ?strategie=SMC|straddle|... cible l'onglet.
    { path: '/simulation', component: () => import('../views/SimulationView.vue') },

    // Rapport d'activité — centre d'analyse des stratégies (04/09).
    // ?strategie=SMC|straddle|rockets cible l'onglet (bloc dashboard).
    { path: '/analyses', component: () => import('../views/AnalysesView.vue') },
    { path: '/straddle/analyse', redirect: '/analyses?strategie=straddle' }, // fusion 23/09 : le rapport d'activité est LA page d'analyse
    { path: '/smc/analyse', redirect: '/analyses?strategie=SMC' }, // fusion 23/09 : le rapport d'activité est LA page d'analyse
    { path: '/rockets/analyse', redirect: '/analyses?strategie=rockets' }, // fusion 23/09 : le rapport d'activité est LA page d'analyse
    { path: '/kdj/analyse', redirect: '/analyses?strategie=kdj_halftrend' }, // fusion 23/09 : le rapport d'activité est LA page d'analyse

    // Fonctionnalités IA — page à 3 onglets (14/09) : les anciennes routes
    // /ia/ml et /ia/llm redirigent vers l'onglet correspondant.
    { path: '/ia', component: () => import('../views/IaView.vue') },
    { path: '/ia/ml',  redirect: { path: '/ia', query: { onglet: 'ml' } } },
    { path: '/ia/llm', redirect: { path: '/ia', query: { onglet: 'ml' } } }, // fusion 23/09

    // Presse
    { path: '/presse', component: () => import('../views/PresseView.vue') },

    // Données — pilotage du pipeline, risque par actif, connexions.
    // Paramètres — réglages des stratégies (bouton ⚙️ de chaque page
    // stratégie ; ?strategie=SMC n'en montre qu'une).
    { path: '/donnees',    component: () => import('../views/DonneesView.vue') },
  ]
})

export default router
