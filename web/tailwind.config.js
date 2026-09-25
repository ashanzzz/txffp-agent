/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: '#f8fafc',
        panel: '#ffffff',
        border: '#e2e8f0',
        primary: {
          DEFAULT: '#2563eb',
          hover: '#1d4ed8',
        }
      }
    },
  },
  plugins: [],
}
