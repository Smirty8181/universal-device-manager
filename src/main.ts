import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import "./styles/main.css";

const app = createApp(App).use(createPinia()).use(router);
router.isReady().then(() => app.mount("#app"));
