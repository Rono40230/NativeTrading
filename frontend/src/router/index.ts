import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: () => import('../views/DashboardHome.vue') },
    { path: '/charts', component: () => import('../views/ChartsView.vue') },
    { path: '/pnl', component: () => import('../views/PnLView.vue') },
    { path: '/history', component: () => import('../views/HistoryView.vue') },
    { path: '/settings', component: () => import('../views/SettingsView.vue') },
    { path: '/heatmap', component: () => import('../views/HeatmapView.vue') }
  ]
})

export default router
