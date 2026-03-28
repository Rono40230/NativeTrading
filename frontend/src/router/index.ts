import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    // Dashboard
    { path: '/', component: () => import('../views/DashboardHome.vue') },

    // Stratégies
    { path: '/rockets',             component: () => import('../views/RocketsView.vue') },
    { path: '/smc',                 component: () => import('../views/SmcView.vue') },
    { path: '/smc/analyser',        component: () => import('../views/SMCAnalyzerView.vue') },
    { path: '/smc/graphiques',      component: () => import('../views/ChartsView.vue') },
    { path: '/straddle',            component: () => import('../views/StraddleView.vue') },
    { path: '/straddle/signaux',    component: () => import('../views/StraddleSignauxView.vue') },
    { path: '/straddle/backtest',   component: () => import('../views/StraddleBacktestView.vue') },

    // Performance
    { path: '/pnl',      component: () => import('../views/PnLView.vue') },
    { path: '/history',  component: () => import('../views/HistoryView.vue') },
    { path: '/heatmap',  component: () => import('../views/HeatmapView.vue') },

    // Outils IA
    { path: '/ia/chart', component: () => import('../views/ChartImportView.vue') },
    { path: '/ia/coach', component: () => import('../views/SMCCoachView.vue') },
    { path: '/lexique',  component: () => import('../views/LexiqueView.vue') },

    // Système
    { path: '/data',     component: () => import('../views/DataManagementView.vue') },
    { path: '/settings', component: () => import('../views/SettingsView.vue') },
  ]
})

export default router
