import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

const apiTarget = process.env.VITE_API_TARGET ?? "http://127.0.0.1:8080";

export default defineConfig({
  plugins: [vue()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }
          if (
            id.includes("/node_modules/@codemirror/") ||
            id.includes("/node_modules/codemirror/")
          ) {
            return "vendor-codemirror";
          }
          if (
            id.includes("/node_modules/element-plus/") ||
            id.includes("/node_modules/@element-plus/")
          ) {
            return "vendor-element-plus";
          }
          if (
            id.includes("/node_modules/vue/") ||
            id.includes("/node_modules/vue-router/") ||
            id.includes("/node_modules/pinia/")
          ) {
            return "vendor-vue";
          }
          return "vendor";
        },
      },
    },
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: apiTarget,
        changeOrigin: true,
        xfwd: true,
      },
    },
  },
  preview: {
    proxy: {
      "/api": {
        target: apiTarget,
        changeOrigin: true,
        xfwd: true,
      },
    },
  },
});
