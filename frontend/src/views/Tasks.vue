<script setup>
import { ref, onMounted } from "vue";
import { api } from "../api.js";

const logs = ref([]);
const loading = ref(false);
const showDetail = ref(false);
const detailLog = ref(null);

async function load() {
  loading.value = true;
  try { logs.value = await api.listTasks(); } finally { loading.value = false; }
}

function statusClass(s) {
  return { success: "badge-green", failed: "badge-red", running: "badge-blue", pending: "badge-gray" }[s] || "badge-gray";
}

function formatTime(iso) { return iso ? new Date(iso).toLocaleString("zh-CN") : "-"; }

function openDetail(log) {
  detailLog.value = log;
  showDetail.value = true;
}

function prettyJson(str) {
  try { return JSON.stringify(JSON.parse(str), null, 2); } catch { return str || "-"; }
}

onMounted(load);
</script>

<template>
  <div>
    <header><h1>任务监控</h1><p class="sub">场景执行记录与详情分析</p></header>
    <section class="toolbar">
      <button @click="load" :disabled="loading">{{ loading ? "刷新中…" : "刷新" }}</button>
    </section>

    <!-- 详情弹窗 -->
    <div v-if="showDetail" class="modal-overlay" @click.self="showDetail = false">
      <div class="modal">
        <h3>任务详情 #{{ detailLog.id }}</h3>
        <div class="detail-body">
          <div class="detail-row">
            <span class="detail-label">场景</span>
            <span class="detail-value">{{ detailLog.scenario_name }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">状态</span>
            <span :class="['badge', statusClass(detailLog.status)]">{{ detailLog.status }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">开始时间</span>
            <span class="detail-value">{{ formatTime(detailLog.started_at) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">完成时间</span>
            <span class="detail-value">{{ formatTime(detailLog.completed_at) }}</span>
          </div>
          <div v-if="detailLog.error_msg" class="detail-section">
            <div class="detail-label" style="color:#e06060">错误信息</div>
            <pre class="detail-code error">{{ detailLog.error_msg }}</pre>
          </div>
          <div v-if="detailLog.result" class="detail-section">
            <div class="detail-label">执行结果</div>
            <pre class="detail-code">{{ prettyJson(detailLog.result) }}</pre>
          </div>
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" @click="showDetail = false">关闭</button>
        </div>
      </div>
    </div>

    <table class="data-table">
      <thead><tr><th>ID</th><th>场景</th><th>状态</th><th>开始时间</th><th>完成时间</th><th>结果</th></tr></thead>
      <tbody>
        <tr v-for="log in logs" :key="log.id" @click="openDetail(log)" class="clickable">
          <td>{{ log.id }}</td>
          <td>{{ log.scenario_name }}</td>
          <td><span :class="['badge', statusClass(log.status)]">{{ log.status }}</span></td>
          <td>{{ formatTime(log.started_at) }}</td>
          <td>{{ formatTime(log.completed_at) }}</td>
          <td>
            <span v-if="log.error_msg" class="error-text" :title="log.error_msg">{{ log.error_msg.slice(0, 40) }}...</span>
            <span v-else-if="log.result" class="success-text">已完成</span>
            <span v-else>-</span>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="!logs.length" class="empty">暂无任务记录</p>
  </div>
</template>

<style scoped>
header h1 { font-size: 20px; margin: 0 0 4px; }
.sub { color: #888; font-size: 13px; margin: 0 0 16px; }
.toolbar { display: flex; gap: 10px; margin-bottom: 16px; }
button { background: var(--green); color: #10240a; border: none; padding: 8px 16px; border-radius: 6px; font-weight: 600; cursor: pointer; }
button:disabled { opacity: 0.6; cursor: default; }
.btn-secondary { background: #3a3a3a; color: #e8e8e8; }
.data-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.data-table th, .data-table td { padding: 10px 12px; text-align: left; border-bottom: 1px solid var(--border); }
.data-table th { color: #888; font-weight: 500; }
.data-table tr.clickable { cursor: pointer; transition: background 0.1s; }
.data-table tr.clickable:hover { background: rgba(124,179,66,0.05); }
.badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; text-transform: uppercase; }
.badge-green { background: rgba(124,179,66,0.2); color: var(--green); }
.badge-red { background: rgba(200,60,60,0.2); color: #e06060; }
.badge-blue { background: rgba(80,140,220,0.2); color: #6a9ee0; }
.badge-gray { background: #333; color: #888; }
.error-text { color: #e06060; font-size: 12px; }
.success-text { color: var(--green); font-size: 12px; }
.empty { color: #666; padding: 40px; text-align: center; }

.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; overflow: auto; padding: 40px 0; }
.modal { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 24px; width: 720px; max-width: 90vw; max-height: 85vh; display: flex; flex-direction: column; }
.modal h3 { margin: 0 0 16px; font-size: 16px; }
.detail-body { flex: 1; overflow: auto; }
.detail-row { display: flex; gap: 12px; margin-bottom: 10px; font-size: 13px; }
.detail-label { color: #888; min-width: 80px; }
.detail-value { color: #e8e8e8; }
.detail-section { margin-top: 12px; }
.detail-code { background: #1a1a1a; border: 1px solid #333; border-radius: 6px; padding: 12px; font-size: 12px; font-family: "SF Mono", "Consolas", monospace; color: #ccc; white-space: pre-wrap; word-break: break-all; max-height: 300px; overflow: auto; }
.detail-code.error { color: #e06060; border-color: rgba(200,60,60,0.3); }
.modal-actions { display: flex; justify-content: flex-end; margin-top: 16px; }
</style>
