import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";

import App from "./App.vue";
import { routes } from "./app/router/routes";
import { setupGuards } from "./app/router/guards";
import { setupElementPlus } from "./app/element-plus";
import { setupRouterPerformance } from "./app/performance";

import "./app/styles/tokens.css";
import "./app/styles/base.css";

const pinia = createPinia();

const router = createRouter({
  history: createWebHistory(),
  routes,
});

setupRouterPerformance(router);
setupGuards(router);

const app = createApp(App);
app.use(pinia);
app.use(router);
setupElementPlus(app);
app.mount("#app");
