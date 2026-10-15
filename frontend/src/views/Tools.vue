<script setup>
import { ref, onMounted } from "vue";
import { api } from "../api.js";

const tools = ref([]);
const showForm = ref(false);
const showDiscover = ref(false);
const showTest = ref(false);
const testResult = ref(null);
const editing = ref(null);
const discovering = ref(false);
const discovered = ref([]);
const discoverTimeout = ref(5);
const discoverHint = ref("");
const form = ref({
  id: 0, tool_type: "onvif_camera", name: "", endpoint: "", xaddr: "",
  location: "", username: "", password: "", enabled: true,
});

async function load() {
  tools.value = await api.listTools();
}

function addNew() {
  form.value = { id: 0, tool_type: "onvif_camera", name: "", endpoint: "http://192.168.1.100:8000/onvif/device_service", xaddr: "", location: "", username: "", password: "", enabled: true };
  editing.value = null;
  showForm.value = true;
}

function edit(t) {
  form.value = JSON.parse(JSON.stringify(t));
  editing.value = t.id;
  showForm.value = true;
}

async function save() {
  try {
    if (editing.value) await api.updateTool(editing.value, form.value);
    else await api.addTool(form.value);
    showForm.value = false;
    await load();
  } catch (e) { alert("保存失败: " + e); }
}

async function remove(id) {
  if (!confirm("确定删除？")) return;
  try { await api.deleteTool(id); await load(); } catch (e) { alert(e); }
}

async function openDiscover() {
  showDiscover.value = true;
  discovered.value = [];
  await doDiscover();
}

async function doDiscover() {
  discovering.value = true;
  discoverHint.value = "";
  try {
    discovered.value = await api.discoverTools(discoverTimeout.value);
    if (!discovered.value.length) {
      discoverHint.value = "未扫描到设备。可能原因：1) 摄像头未开启 ONVIF/WS-Discovery；2) 与当前设备不在同一网段；3) macOS 防火墙拦截了 UDP 多播。建议直接手动添加接入。";
    }
  } catch (e) { alert("扫描失败: " + e); }
  finally { discovering.value = false; }
}

function selectDiscovered(device) {
  form.value.endpoint = device.endpoint;
  form.value.xaddr = device.xaddr || "";
  form.value.name = device.name || device.endpoint;
  showDiscover.value = false;
  showForm.value = true;
}

async function testTool(id) {
  try {
    testResult.value = null;
    showTest.value = true;
    const result = await api.testSnapshot(id);
    testResult.value = result;
  } catch (e) {
    alert("测试失败: " + e);
    showTest.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div>
    <header><h1>接入管理</h1><p class="sub">ONVIF 摄像头接入 — 添加、配置、测试抓图</p></header>
    <section class="toolbar">
      <button @click="addNew">+ 添加工具</button>
      <button class="btn-secondary" @click="openDiscover">扫描局域网</button>
      <button class="btn-secondary" @click="load">刷新</button>
    </section>

    <!-- 添加/编辑表单 -->
    <div v-if="showForm" class="modal-overlay" @click.self="showForm = false">
      <div class="modal">
        <h3>{{ editing ? "编辑工具" : "添加工具" }}</h3>
        <div class="form-grid">
          <div class="field"><label>名称</label><input v-model="form.name" /></div>
          <div class="field"><label>类型</label>
            <select v-model="form.tool_type"><option value="onvif_camera">ONVIF 摄像头</option></select>
          </div>
          <div class="field full"><label>Endpoint</label><input v-model="form.endpoint" placeholder="http://192.168.1.100:8000/onvif/device_service" /></div>
          <div class="field"><label>XAddr</label><input v-model="form.xaddr" placeholder="可选" /></div>
          <div class="field"><label>位置</label><input v-model="form.location" placeholder="如：仓库A-货架区" /></div>
          <div class="field"><label>账号</label><input v-model="form.username" placeholder="admin" /></div>
          <div class="field"><label>密码</label><input v-model="form.password" type="password" placeholder="密码" /></div>
          <div class="field"><label>启用</label><input type="checkbox" v-model="form.enabled" /></div>
        </div>
        <div class="modal-actions">
          <button @click="save">保存</button>
          <button class="btn-secondary" @click="showForm = false">取消</button>
        </div>
      </div>
    </div>

    <!-- 扫描发现弹窗 -->
    <div v-if="showDiscover" class="modal-overlay" @click.self="showDiscover = false">
      <div class="modal modal-wide">
        <h3>扫描局域网 ONVIF 设备</h3>
        <div class="toolbar" style="margin-bottom:12px; align-items:center;">
          <button :disabled="discovering" @click="doDiscover">{{ discovering ? "扫描中…" : "重新扫描" }}</button>
          <div class="field" style="margin:0; flex-direction:row; align-items:center; gap:8px;">
            <label style="font-size:13px; color:#888;">超时</label>
            <select v-model="discoverTimeout" style="width:80px;">
              <option :value="3">3秒</option>
              <option :value="5">5秒</option>
              <option :value="10">10秒</option>
            </select>
          </div>
        </div>
        <div v-if="discoverHint" class="hint-box">{{ discoverHint }}</div>
        <div v-if="discovered.length" class="device-list">
          <div v-for="d in discovered" :key="d.endpoint" class="device-card" @click="selectDiscovered(d)">
            <div style="display:flex; align-items:center; gap:8px; margin-bottom:4px;">
              <div class="device-name">{{ d.name || "未命名设备" }}</div>
              <span v-if="d.source === 'port_scan'" class="badge badge-yellow">端口扫描</span>
              <span v-else-if="d.source === 'ws_discovery'" class="badge badge-blue">WS-Discovery</span>
            </div>
            <div class="device-info">{{ d.endpoint }}</div>
            <div class="device-info">{{ d.xaddr || "" }}</div>
          </div>
        </div>
        <p v-else-if="!discovering" class="empty">未扫描到设备</p>
      </div>
    </div>

    <!-- 测试抓图弹窗 -->
    <div v-if="showTest" class="modal-overlay" @click.self="showTest = false">
      <div class="modal">
        <h3>测试抓图</h3>
        <div v-if="testResult" class="test-result">
          <p>工具: {{ testResult.tool_name }}</p>
          <img v-if="testResult.image_base64" :src="'data:image/jpeg;base64,' + testResult.image_base64" style="max-width:100%; border-radius:8px; margin-top:12px;" />
        </div>
        <p v-else class="empty">正在抓取…</p>
        <div class="modal-actions">
          <button class="btn-secondary" @click="showTest = false">关闭</button>
        </div>
      </div>
    </div>

    <!-- 工具列表 -->
    <div class="tool-list">
      <div v-for="t in tools" :key="t.id" class="tool-card">
        <div class="tool-header">
          <div>
            <span class="tool-name">{{ t.name }}</span>
            <span :class="['badge', t.enabled ? 'badge-green' : 'badge-gray']">{{ t.enabled ? "启用" : "禁用" }}</span>
            <span class="badge badge-blue">{{ t.tool_type }}</span>
          </div>
          <div class="tool-actions">
            <button class="btn-small" @click="testTool(t.id)">测试抓图</button>
            <button class="btn-small" @click="edit(t)">编辑</button>
            <button class="btn-small btn-danger" @click="remove(t.id)">删除</button>
          </div>
        </div>
        <div class="tool-body">
          <div class="info-row"><span class="info-label">Endpoint:</span><span class="info-value">{{ t.endpoint }}</span></div>
          <div class="info-row"><span class="info-label">位置:</span><span class="info-value">{{ t.location || "-" }}</span></div>
          <div class="info-row"><span class="info-label">账号:</span><span class="info-value">{{ t.username || "无" }}</span></div>
        </div>
      </div>
    </div>
    <p v-if="!tools.length" class="empty">暂无工具</p>
  </div>
</template>

<style scoped>
header h1 { font-size: 20px; margin: 0 0 4px; }
.sub { color: #888; font-size: 13px; margin: 0 0 16px; }
.toolbar { display: flex; gap: 10px; margin-bottom: 16px; }
button { background: var(--green); color: #10240a; border: none; padding: 8px 16px; border-radius: 6px; font-weight: 600; cursor: pointer; }
button:disabled { opacity: 0.6; cursor: default; }
.btn-secondary { background: #3a3a3a; color: #e8e8e8; }
.btn-small { padding: 4px 10px; font-size: 12px; margin-right: 6px; }
.btn-danger { background: #8b2e2e; color: #fff; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; overflow: auto; padding: 40px 0; }
.modal { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 24px; width: 640px; max-width: 90vw; }
.modal-wide { width: 720px; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 16px; }
.field { display: flex; flex-direction: column; gap: 4px; }
.field label { font-size: 12px; color: #888; }
.field input, .field select { background: #1e1e1e; border: 1px solid #3a3a3a; color: #e8e8e8; padding: 8px; border-radius: 6px; font-size: 13px; }
.field.full { grid-column: 1 / -1; }
.modal-actions { display: flex; gap: 10px; justify-content: flex-end; margin-top: 16px; }
.device-list { display: flex; flex-direction: column; gap: 8px; }
.device-card { background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 8px; padding: 12px; cursor: pointer; transition: border-color 0.15s; }
.device-card:hover { border-color: var(--green); }
.device-name { font-weight: 600; font-size: 14px; margin-bottom: 4px; }
.device-info { font-size: 12px; color: #888; }
.test-result { text-align: center; }
.tool-list { display: flex; flex-direction: column; gap: 12px; }
.tool-card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 10px; padding: 16px; }
.tool-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.tool-name { font-weight: 600; font-size: 15px; margin-right: 10px; }
.tool-body { display: grid; grid-template-columns: 1fr; gap: 6px; }
.info-row { display: flex; gap: 6px; font-size: 13px; }
.info-label { color: #888; min-width: 60px; }
.info-value { color: #bbb; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-right: 6px; }
.badge-green { background: rgba(124,179,66,0.2); color: var(--green); }
.badge-gray { background: #333; color: #888; }
.badge-blue { background: rgba(66,133,244,0.2); color: #4285f4; }
.badge-yellow { background: rgba(255,193,7,0.15); color: #ffc107; }
.empty { color: #666; padding: 40px; text-align: center; }
.hint-box { background: rgba(255,193,7,0.1); border: 1px solid rgba(255,193,7,0.3); border-radius: 8px; padding: 12px 16px; font-size: 13px; color: #ffc107; margin-bottom: 16px; line-height: 1.6; }
</style>
