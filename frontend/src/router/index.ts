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

    // Outils IA — regroupés en une page à onglets (refonte navigation 01/09) ;
    // les anciens chemins redirigent vers l'onglet correspondant.
    { path: '/ia',            component: () => import('../views/IaView.vue') },
    { path: '/ia/chart',      redirect: { path: '/ia', query: { tab: 'chart' } } },
    { path: '/ia/coach',      redirect: { path: '/ia', query: { tab: 'coach' } } },
    { path: '/config/prompts', redirect: { path: '/ia', query: { tab: 'prompts' } } },
    { path: '/ml-insights', redirect: { path: '/ia', query: { tab: 'prompts' } } },

    // Presse
    { path: '/presse', component: () => import('../views/PresseView.vue') },

    // Système — onglets Paramètres + Données, anciens chemins redirigés.
    { path: '/systeme',  component: () => import('../views/SystemeView.vue') },
    { path: '/settings', redirect: { path: '/systeme', query: { tab: 'settings' } } },
    { path: '/data',     redirect: { path: '/systeme', query: { tab: 'data' } } },

    // Historique global retiré (redondant avec les historiques par stratégie) —
    // l'ancien chemin renvoie au dashboard.
    { path: '/history',  redirect: { path: '/' } },
  ]
})

export default router
