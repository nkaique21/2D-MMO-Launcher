/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        launcher: {
          bg: '#070710',
          panel: '#10101c',
          panelSoft: '#171427',
          border: '#2a2141',
          purple: '#8b5cf6',
          purpleDeep: '#5b21b6',
          text: '#f4f0ff',
          muted: '#a99bc4',
        },
      },
      boxShadow: {
        glow: '0 0 48px rgba(139, 92, 246, 0.28)',
      },
    },
  },
  plugins: [],
};
