<script setup>
import { ref, onMounted } from "vue";
import { api } from "../api.js";

const credentials = ref([]);
const showForm = ref(false);
const form = ref({ id: "", service: "", username: "", password: "" });

async function load() { credentials.value = await api.listCredentials(); }

function addNew() { form.value = { id: "", service: "", username: "", password: "" }; showForm.value = true; }

async function save() {
  try { await api.saveCredential(form.value); showForm.value = false; await load(); }
  catch (e) { alert("保存失败: " + e); }
}

async function remove(cred) {
  if (!confirm("确定删除此凭据？")) return;
  try { await api.deleteCredential(cred.id, cred.username); await load(); }
  catch (e) { alert("删除失败: " + e); }
}

onMounted(load);
</script>

<template>
  <div>
    <header><h1>凭据管理</h1><p class="sub">摄像头登录凭据（密码存系统钥匙串）</p></header>
    <section class="toolbar">
      <button @click="addNew">+ 添加凭据</button>
      <button class="btn-secondary" @click="load">刷新</button>
    </section>

    <div v-if="showForm" class="modal-overlay" @click.self="showForm = false">
      <div class="modal">
        <h3>添加凭据</h3>
        <div class="form-grid">
          <div class="field"><label>凭据ID</label><input v-model="form.id" placeholder="如：cam-admin" /></div>
          <div class="field"><label>服务标识</label><input v-model="form.service" placeholder="如：hikvision" /></div>
          <div class="field"><label>用户名</label><input v-model="form.username" placeholder="admin" /></div>
          <div class="field"><label>密码</label><input type="password" v-model="form.password" /></div>
        </div>
        <div class="modal-actions">
          <button @click="save">保存</button>
          <button class="btn-secondary" @click="showForm = false">取消</button>
        </div>
      </div>
    </div>

    <table class="data-table">
      <thead><tr><th>ID</th><th>服务</th><th>用户名</th><th>创建时间</th><th>操作</th></tr></thead>
      <tbody>
        <tr v-for="c in credentials" :key="c.id">
          <td>{{ c.id }}</td>
          <td>{{ c.service }}</td>
          <td>{{ c.username }}</td>
          <td>{{ new Date(c.created_at).toLocaleString() }}</td>
          <td><button class="btn-small btn-danger" @click="remove(c)">删除</button></td>
        </tr>
      </tbody>
    </table>
    <p v-if="!credentials.length" class="empty">暂无凭据</p>
  </div>
</template>

<style scoped>
header h1 { font-size: 20px; margin: 0 0 4px; }
.sub { color: #888; font-size: 13px; margin: 0 0 16px; }
.toolbar { display: flex; gap: 10px; margin-bottom: 16px; }
button { background: var(--green); color: #10240a; border: none; padding: 8px 16px; border-radius: 6px; font-weight: 600; cursor: pointer; }
.btn-secondary { background: #3a3a3a; color: #e8e8e8; }
.btn-small { padding: 4px 10px; font-size: 12px; }
.btn-danger { background: #8b2e2e; color: #fff; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 24px; width: 480px; max-width: 90vw; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 16px 0; }
.field { display: flex; flex-direction: column; gap: 4px; }
.field label { font-size: 12px; color: #888; }
.field input { background: #1e1e1e; border: 1px solid #3a3a3a; color: #e8e8e8; padding: 8px; border-radius: 6px; }
.modal-actions { display: flex; gap: 10px; justify-content: flex-end; }
.data-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.data-table th, .data-table td { padding: 10px 12px; text-align: left; border-bottom: 1px solid var(--border); }
.empty { color: #666; padding: 40px; text-align: center; }
</style>
