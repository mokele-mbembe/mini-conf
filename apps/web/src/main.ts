import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import ElementPlus from "element-plus";
import "element-plus/dist/index.css";

import App from "./App.vue";
import { routes } from "./app/router/routes";
import { setupGuards } from "./app/router/guards";

import "./app/styles/tokens.css";
import "./app/styles/base.css";

const pinia = createPinia();

const router = createRouter({
  history: createWebHistory(),
  routes,
});

setupGuards(router);

const app = createApp(App);
app.use(pinia);
app.use(router);
app.use(ElementPlus);
app.mount("#app");
