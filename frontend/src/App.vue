<template>
  <div id="app" class="min-h-screen bg-gray-900 text-white">
    <NavBar />
    <main class="container mx-auto px-4 py-6">
      <RouterView />
    </main>
    <ToastAlerte />
  </div>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router'
import NavBar from './components/common/NavBar.vue'
import ToastAlerte from './components/common/ToastAlerte.vue'
</script>
