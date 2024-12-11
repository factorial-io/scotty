import daisyui from "daisyui";
import typography from "@tailwindcss/typography"

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,svelte,js,ts}'],
  theme: {
    extend: {},
  },
  daisyui: {
      themes: [
        {
          mytheme: {
            "primary": "#e11d48",
            "secondary": "#4f46e5",
            "accent": "#60a5fa",
            "neutral": "#9ca3af",
            "base-100": "#ffffff",
            "info": "#6b7280",
            "success": "#059669",
            "warning": "#f59e0b",
            "error": "#e11d48",
            },
          },
          "dark",
        ],
      },
  plugins: [typography, daisyui],
}
