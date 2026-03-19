/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  safelist: ["grid-cols-12", "xl:grid-cols-12"],
  theme: { extend: {} },
  plugins: [],
}
