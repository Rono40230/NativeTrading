<template>
  <div id="app" class="flex min-h-screen bg-gray-900 text-white">
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
