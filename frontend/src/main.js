import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import Tools from "./views/Tools.vue";
import Scenarios from "./views/Scenarios.vue";
import Tasks from "./views/Tasks.vue";

const routes = [
  { path: "/", redirect: "/tools" },
  { path: "/tools", component: Tools },
  { path: "/scenarios", component: Scenarios },
  { path: "/tasks", component: Tasks },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

createApp(App).use(router).mount("#app");
