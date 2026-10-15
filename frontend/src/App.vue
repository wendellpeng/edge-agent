<script setup>
import { useRoute } from "vue-router";
const route = useRoute();

const navItems = [
  { path: "/tools", label: "接入管理" },
  { path: "/scenarios", label: "场景编排" },
  { path: "/tasks", label: "任务监控" },
];
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <div class="logo">
        <div class="logo-icon"></div>
        <div class="logo-text">视觉巡检智能体</div>
      </div>
      <nav>
        <router-link
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          :class="['nav-item', { active: route.path === item.path }]"
        >
          {{ item.label }}
        </router-link>
      </nav>
      <div class="footer">
        <span class="status-dot"></span>
        运行中
      </div>
    </aside>
    <main class="content">
      <router-view />
    </main>
  </div>
</template>

<style>
:root {
  --green: #7CB342;
  --bg: #1a1a1a;
  --sidebar-bg: #141414;
  --card-bg: #242424;
  --border: #333;
  --text: #e8e8e8;
  --text-muted: #888;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
  background: var(--bg);
  color: var(--text);
}
.layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}
.sidebar {
  width: 200px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}
.logo {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 20px 16px;
  border-bottom: 1px solid var(--border);
}
.logo-icon {
  width: 28px;
  height: 28px;
  background: var(--green);
  border-radius: 6px;
}
.logo-text {
  font-size: 15px;
  font-weight: 600;
}
nav {
  flex: 1;
  padding: 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.nav-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 8px;
  color: var(--text-muted);
  text-decoration: none;
  font-size: 14px;
  transition: all 0.15s;
}
.nav-item:hover {
  background: rgba(124, 179, 66, 0.08);
  color: var(--text);
}
.nav-item.active {
  background: rgba(124, 179, 66, 0.15);
  color: var(--green);
}
.footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 8px;
}
.status-dot {
  width: 8px;
  height: 8px;
  background: var(--green);
  border-radius: 50%;
}
.content {
  flex: 1;
  overflow: auto;
  padding: 24px;
}
</style>
