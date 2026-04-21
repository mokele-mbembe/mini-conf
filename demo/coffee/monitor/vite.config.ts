import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5174,
    host: "127.0.0.1",
    proxy: {
      // Forward /api/demo/* to the demo control API
      "/api/demo": {
        target: "http://127.0.0.1:19010",
        changeOrigin: false,
      },
    },
  },
});
