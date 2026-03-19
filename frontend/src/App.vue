<template>
  <div id="app" class="flex h-screen bg-gray-900 text-white">
    <!-- Zone de déclenchement invisible sur le bord gauche -->
    <div class="sidebar-trigger fixed left-0 top-0 h-full w-3 z-50" />
    <SideBar />
    <main class="flex-1 overflow-y-auto px-4 py-6">
      <RouterView />
    </main>
    <ToastAlerte />
  </div>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router'
import SideBar from './components/common/SideBar.vue'
import ToastAlerte from './components/common/ToastAlerte.vue'
</script>

<style>
/* La sidebar est par défaut hors-écran, slide-in au survol de la zone trigger */
.sidebar-trigger:hover ~ aside,
aside:hover {
  transform: translateX(0) !important;
  opacity: 1 !important;
}
</style>
