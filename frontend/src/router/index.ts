import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: () => import('../views/DashboardHome.vue') },
    { path: '/charts', component: () => import('../views/ChartsView.vue') },
    { path: '/pnl', component: () => import('../views/PnLView.vue') },
    { path: '/history', component: () => import('../views/HistoryView.vue') },
    { path: '/settings', component: () => import('../views/SettingsView.vue') },
    { path: '/heatmap', component: () => import('../views/HeatmapView.vue') },
    { path: '/ia/analyser', component: () => import('../views/SMCAnalyzerView.vue') },
    { path: '/ia/chart', component: () => import('../views/ChartImportView.vue') },
    { path: '/ia/coach', component: () => import('../views/SMCCoachView.vue') },
    { path: '/lexique',   component: () => import('../views/LexiqueView.vue') },
    { path: '/data', component: () => import('../views/DataManagementView.vue') },
  ]
})

export default router
