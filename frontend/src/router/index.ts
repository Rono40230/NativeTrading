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
    { path: '/history',  component: () => import('../views/HistoryView.vue') },
    { path: '/heatmap',  component: () => import('../views/HeatmapView.vue') },

    // Outils IA
    { path: '/ia/chart',    component: () => import('../views/ChartImportView.vue') },
    { path: '/ia/coach',    component: () => import('../views/SMCCoachView.vue') },
    { path: '/ml-insights', component: () => import('../views/MlInsightsView.vue') },

    // Presse
    { path: '/presse', component: () => import('../views/PresseView.vue') },

    // Système
    { path: '/data',            component: () => import('../views/DataManagementView.vue') },
    { path: '/settings',        component: () => import('../views/SettingsView.vue') },
    { path: '/config/prompts',  component: () => import('../views/PromptsIAView.vue') },
  ]
})

export default router
