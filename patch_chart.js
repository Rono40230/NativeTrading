const fs = require('fs');

const files = [
  '/mnt/IA/native-trading-ai/frontend/src/components/common/SmcEquityChart.vue',
  '/mnt/IA/native-trading-ai/frontend/src/components/common/StraddleEquityChart.vue',
  '/mnt/IA/native-trading-ai/frontend/src/components/common/RocketsEquityChart.vue'
];

for (const file of files) {
  let content = fs.readFileSync(file, 'utf8');

// Supprimer le <div v-else-if
content = content.replace(
  /<div v-else-if="!data \|\| data\.points\.length === 0"[ \S\n]*?Aucune donnée\n\s*<\/div>/,
  ''
);
// Enlever le v-else sur le svg
content = content.replace(/<svg v-else :viewBox=/, '<svg v-else-if="data" :viewBox=');

// Dans le setup, utiliser dummy points
content = content.replace(
  /const linePath = computed\(\(\) => \{\n\s*if \(\!data\.value\?\.points\.length\) return ''\n\s*const pts = data\.value\.points/,
  `const linePath = computed(() => {
  if (!data.value) return ''
  const orig = data.value.points
  const pts = orig.length > 1 ? orig : orig.length === 1 ? [{ equity_cumulee: data.value.capital_initial }, orig[0]] : [{ equity_cumulee: data.value.capital_initial }, { equity_cumulee: data.value.capital_initial }]`
);

content = content.replace(
  /const areaPath = computed\(\(\) => \{\n\s*if \(\!data\.value\?\.points\.length\) return ''\n\s*const pts = data\.value\.points/,
  `const areaPath = computed(() => {
  if (!data.value) return ''
  const orig = data.value.points
  const pts = orig.length > 1 ? orig : orig.length === 1 ? [{ equity_cumulee: data.value.capital_initial }, orig[0]] : [{ equity_cumulee: data.value.capital_initial }, { equity_cumulee: data.value.capital_initial }]`
);

content = content.replace(
  /data\.value\?\.points\.length/g,
  '((data.value?.points?.length > 1) ? data.value.points.length : 2)'
);

fs.writeFileSync(file, content, 'utf8');
}
