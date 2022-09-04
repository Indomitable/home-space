import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import { provideAppServices } from "./di/app.provide";

import "./assets/main.scss";

const app = createApp(App);
const router = provideAppServices(app);

app.use(createPinia());
app.use(router);

app.mount("#app");
