import { createRouter, createWebHashHistory } from "vue-router";

const trayWindow = window.location.hash.startsWith("#/tray");
const overlayWindow = window.location.hash.startsWith("#/overlay");

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "dashboard", component: () => import("./views/DashboardView.vue") },
    { path: "/devices", name: "devices", component: () => import("./views/DevicesView.vue") },
    { path: "/devices/:id", name: "device", component: () => import("./views/DeviceView.vue") },
    { path: "/rgb", name: "rgb", component: () => import("./views/RgbStudioView.vue") },
    { path: "/keyboard", name: "keyboard", component: () => import("./views/StudioView.vue") },
    { path: "/mouse", name: "mouse", component: () => import("./views/StudioView.vue") },
    { path: "/audio", name: "audio", component: () => import("./views/StudioView.vue") },
    { path: "/input", name: "input", component: () => import("./views/InputView.vue") },
    { path: "/macros", name: "macros", component: () => import("./views/MacrosView.vue") },
    { path: "/profiles", name: "profiles", component: () => import("./views/ProfilesView.vue") },
    { path: "/database", name: "database", component: () => import("./views/DatabaseView.vue") },
    { path: "/drivers", name: "drivers", component: () => import("./views/DriversView.vue") },
    { path: "/diagnostics", name: "diagnostics", component: () => import("./views/DiagnosticsView.vue") },
    { path: "/settings", name: "settings", component: () => import("./views/SettingsView.vue") },
    { path: "/visualizer", name: "visualizer", component: () => import("./views/VisualizerView.vue") },
    { path: "/autoclicker", name: "autoclicker", component: () => import("./views/AutoClickerView.vue") },
    { path: "/intelligence", name: "intelligence", component: () => import("./views/IntelligenceView.vue") },
    { path: "/tray", name: "tray", component: () => import("./views/TrayView.vue") },
    { path: "/overlay", name: "overlay", component: () => import("./views/OverlayView.vue") },
  ],
});

if (trayWindow) {
  router.beforeEach((to) => (to.path === "/tray" ? true : "/tray"));
} else if (overlayWindow) {
  router.beforeEach((to) => (to.path === "/overlay" ? true : "/overlay"));
}

export default router;
