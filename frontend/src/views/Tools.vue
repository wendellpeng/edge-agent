<script setup>
import { ref, onMounted, computed } from "vue";
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
const enumerating = ref(false);
const rtspTemplates = ref([]);

// 接入向导
const showWizard = ref(false);
const wizardStep = ref(1); // 1:输入凭据 2:探测中 3:显示结果
const wizardDevice = ref(null);
const wizardProbeResult = ref(null);
const wizardUsername = ref("admin");
const wizardPassword = ref("");
const wizardProbing = ref(false);
const wizardChannels = ref([]);
const wizardSelected = ref([]);
const wizardProbeLog = ref([]);
const showProbeLog = ref(false);

// 手动验证
const manualUrl = ref("");
const manualVerifying = ref(false);
const manualResult = ref(null);

const form = ref({
  id: 0, tool_type: "rtsp_camera", name: "", endpoint: "", xaddr: "",
  location: "", username: "", password: "", channel: 1, brand: "", enabled: true,
});

async function load() {
  tools.value = await api.listTools();
}

async function loadTemplates() {
  try {
    rtspTemplates.value = await api.listRtspTemplates();
  } catch (e) {
    rtspTemplates.value = [];
  }
}

function addNew() {
  form.value = { id: 0, tool_type: "rtsp_camera", name: "", endpoint: "", xaddr: "", location: "", username: "", password: "", channel: 1, brand: "", enabled: true };
  editing.value = null;
  showForm.value = true;
}

function edit(t) {
  form.value = JSON.parse(JSON.stringify(t));
  if (!form.value.channel) form.value.channel = 1;
  if (!form.value.brand) form.value.brand = "";
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
      discoverHint.value = "未扫描到 RTSP 设备。可能原因：1) 设备未开启 RTSP（554端口）；2) 与当前设备不在同一网段；3) 防火墙拦截 554 端口。建议直接手动添加接入。";
    }
  } catch (e) { alert("扫描失败: " + e); }
  finally { discovering.value = false; }
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

function extractIp(str) {
  const m = str.match(/(\d+\.\d+\.\d+\.\d+)/);
  return m ? m[1] : "";
}

// ===================== 接入向导 =====================

function openWizard(device) {
  wizardDevice.value = device;
  wizardStep.value = 1;
  wizardProbeResult.value = null;
  wizardUsername.value = "admin";
  wizardPassword.value = "";
  wizardProbing.value = false;
  wizardChannels.value = [];
  wizardSelected.value = [];
  wizardProbeLog.value = [];
  showProbeLog.value = false;
  showWizard.value = true;
}

async function doProbe() {
  if (!wizardDevice.value) return;
  const ip = extractIp(wizardDevice.value.endpoint) || wizardDevice.value.endpoint;

  wizardStep.value = 2;
  wizardProbing.value = true;
  wizardChannels.value = [];

  try {
    const result = await api.probeDevice({
      ip,
      username: wizardUsername.value,
      password: wizardPassword.value,
    });

    wizardProbeResult.value = result;
    wizardChannels.value = result.channels || [];
    wizardProbeLog.value = result.probe_log || [];
    // 默认选中所有成功的通道
    wizardSelected.value = wizardChannels.value
      .filter(c => c.success)
      .map(c => c.channel);
    wizardStep.value = 3;
  } catch (e) {
    alert("探测失败: " + e);
    wizardStep.value = 1;
  } finally {
    wizardProbing.value = false;
  }
}

async function saveWizardTools() {
  const selected = wizardChannels.value.filter(c => wizardSelected.value.includes(c.channel));
  if (selected.length === 0) {
    alert("请至少选择一个验证成功的通道");
    return;
  }

  const ip = wizardProbeResult.value.ip;
  const brand = wizardProbeResult.value.brand || "未知品牌";

  for (const ch of selected) {
    const tool = {
      tool_type: "rtsp_camera",
      name: `${brand} ${ip} 通道${ch.channel}`,
      endpoint: ch.url,
      xaddr: "",
      location: "",
      username: wizardUsername.value,
      password: wizardPassword.value,
      channel: ch.channel,
      brand: brand,
      enabled: true,
    };
    try {
      await api.addTool(tool);
    } catch (e) {
      console.error("保存工具失败:", e);
    }
  }

  showWizard.value = false;
  await load();
}

function toggleSelect(channel) {
  const idx = wizardSelected.value.indexOf(channel);
  if (idx >= 0) {
    wizardSelected.value.splice(idx, 1);
  } else {
    wizardSelected.value.push(channel);
  }
}

async function verifyManualUrl() {
  if (!manualUrl.value) return;
  manualVerifying.value = true;
  manualResult.value = null;
  try {
    const result = await api.verifyRtspUrl({ url: manualUrl.value });
    manualResult.value = result;
  } catch (e) {
    manualResult.value = { success: false, error: e.message };
  } finally {
    manualVerifying.value = false;
  }
}

function saveManualTool() {
  if (!manualResult.value || !manualResult.value.success) return;
  const url = manualUrl.value;
  // 从URL中提取通道号（如果可能）
  const chMatch = url.match(/channel[=/](\d+)|ch(\d+)|Channels\/(\d+)|stream(\d+)|video(\d+)|\/(\d+)$/);
  const channel = chMatch ? parseInt(chMatch[1] || chMatch[2] || chMatch[3] || chMatch[4] || chMatch[5] || chMatch[6]) : 1;

  const tool = {
    tool_type: "rtsp_camera",
    name: `手动添加 ${extractIp(url) || '未知IP'} 通道${channel}`,
    endpoint: url,
    xaddr: "",
    location: "",
    username: wizardUsername.value,
    password: wizardPassword.value,
    channel: channel,
    brand: "manual",
    enabled: true,
  };
  api.addTool(tool).then(() => {
    showWizard.value = false;
    load();
  }).catch(e => alert("保存失败: " + e));
}

onMounted(() => {
  load();
  loadTemplates();
});
</script>

<template>
  <div>
    <header><h1>接入管理</h1><p class="sub">RTSP 摄像头/NVR 接入 — 添加、配置、测试抓图</p></header>
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
          <!-- 第一行：名称 + 位置 -->
          <div class="field"><label>名称</label><input v-model="form.name" /></div>
          <div class="field"><label>位置</label><input v-model="form.location" placeholder="如：仓库A-货架区" /></div>
          <!-- 第二行：RTSP URL -->
          <div class="field full">
            <label>RTSP URL</label>
            <input v-model="form.endpoint" placeholder="rtsp://admin:pass@192.168.0.104:554/Streaming/Channels/101" />
          </div>
          <!-- 第三行：账号 + 密码 -->
          <div class="field"><label>账号</label><input v-model="form.username" placeholder="admin" /></div>
          <div class="field"><label>密码</label><input v-model="form.password" type="password" placeholder="密码" /></div>
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
        <h3>扫描局域网 RTSP 设备</h3>
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
          <div v-for="d in discovered" :key="d.endpoint" class="device-card">
            <div style="display:flex; align-items:center; justify-content:space-between;">
              <div>
                <div class="device-name">{{ d.name || "RTSP 设备" }}</div>
                <div class="device-info">{{ d.endpoint }}</div>
              </div>
              <span class="badge badge-red">RTSP 554</span>
            </div>
            <div style="margin-top:10px; display:flex; gap:8px;">
              <button class="btn-small btn-wizard" @click="openWizard(d)">接入向导</button>
            </div>
          </div>
        </div>
        <p v-else-if="!discovering" class="empty">未扫描到设备</p>
      </div>
    </div>

    <!-- 接入向导弹窗 -->
    <div v-if="showWizard" class="modal-overlay" @click.self="showWizard = false">
      <div class="modal modal-wide">
        <h3>设备接入向导 — {{ extractIp(wizardDevice?.endpoint) }}</h3>

        <!-- Step 1: 输入凭据 -->
        <div v-if="wizardStep === 1" class="wizard-step">
          <p style="margin-bottom:12px; color:#888; font-size:13px;">检测到 RTSP 设备，请输入账号密码进行通道探测：</p>
          <div class="form-grid">
            <div class="field"><label>账号</label><input v-model="wizardUsername" placeholder="admin" /></div>
            <div class="field"><label>密码</label><input v-model="wizardPassword" type="password" placeholder="密码" /></div>
          </div>
          <div class="modal-actions">
            <button @click="doProbe" :disabled="wizardProbing">{{ wizardProbing ? "探测中…" : "探测通道" }}</button>
            <button class="btn-secondary" @click="showWizard = false">取消</button>
          </div>
        </div>

        <!-- Step 2: 探测中 -->
        <div v-if="wizardStep === 2" class="wizard-step">
          <p class="empty">正在尝试多种品牌模板验证通道，请稍候…</p>
          <div class="progress-bar"><div class="progress-fill"></div></div>
          <p style="text-align:center; color:#888; font-size:12px; margin-top:8px;">先识别品牌，再验证通道 1-16</p>
        </div>

        <!-- Step 3: 显示通道结果 -->
        <div v-if="wizardStep === 3" class="wizard-step">
          <div style="margin-bottom:12px; display:flex; align-items:center; gap:8px; justify-content:space-between;">
            <span style="color:#888; font-size:13px;">
              品牌: <strong style="color:#e8e8e8;">{{ wizardProbeResult?.brand || '未知' }}</strong> |
              成功: <strong style="color:var(--green);">{{ wizardChannels.filter(c=>c.success).length }}</strong> / {{ wizardChannels.length }} 通道
            </span>
            <button class="btn-secondary" style="padding:4px 10px; font-size:12px;" @click="showProbeLog = !showProbeLog">
              {{ showProbeLog ? '隐藏' : '查看' }}探测日志 ({{ wizardProbeLog.length }})
            </button>
          </div>

          <!-- 探测日志面板 -->
          <div v-if="showProbeLog" style="max-height:300px; overflow-y:auto; background:#1a1a1a; border:1px solid #333; border-radius:8px; padding:10px; margin-bottom:12px;">
            <div v-for="(log, idx) in wizardProbeLog" :key="idx" style="font-size:11px; padding:4px 0; border-bottom:1px solid #222;">
              <span :style="log.success ? 'color:var(--green);' : 'color:#e74c3c;'">{{ log.success ? '✅' : '❌' }}</span>
              <span style="color:#888; margin-left:6px;">{{ log.pattern_name }}</span>
              <div style="color:#555; margin-left:20px; margin-top:2px;">{{ log.url }}</div>
              <div v-if="log.error" style="color:#c0392b; margin-left:20px; margin-top:2px;">{{ log.error }}</div>
            </div>
          </div>

          <div class="channel-grid">
            <div v-for="ch in wizardChannels" :key="ch.channel" class="channel-card" :class="{ 'card-success': ch.success, 'card-failed': !ch.success }">
              <div style="display:flex; align-items:center; gap:8px; margin-bottom:8px;">
                <input v-if="ch.success" type="checkbox" :checked="wizardSelected.includes(ch.channel)" @change="toggleSelect(ch.channel)" />
                <span class="device-name">通道 {{ ch.channel }}</span>
                <span v-if="ch.success" class="badge badge-green">成功</span>
                <span v-else class="badge badge-red">失败</span>
              </div>
              <div v-if="ch.success && ch.snapshot_base64" class="snapshot-box">
                <img :src="'data:image/jpeg;base64,' + ch.snapshot_base64" />
              </div>
              <div v-if="!ch.success && ch.error" class="hint-box" style="margin-top:8px; font-size:11px; padding:6px 8px;">{{ ch.error }}</div>
            </div>
          </div>

          <div class="modal-actions">
            <button @click="saveWizardTools" :disabled="wizardSelected.length === 0">保存选中 ({{ wizardSelected.length }})</button>
            <button class="btn-secondary" @click="wizardStep = 1">返回修改凭据</button>
            <button class="btn-secondary" @click="showWizard = false">关闭</button>
          </div>

          <!-- 手动验证兜底：自动探测全部失败时显示 -->
          <div v-if="wizardChannels.filter(c=>c.success).length === 0" style="margin-top:20px; padding:16px; background:#1a1a1a; border:1px solid #333; border-radius:8px;">
            <h4 style="margin:0 0 10px; color:#e8e8e8; font-size:14px;">自动探测未成功 — 手动验证</h4>
            <p style="color:#888; font-size:12px; margin-bottom:10px;">如果已知设备的 RTSP URL，可直接输入验证：</p>
            <div style="display:flex; gap:8px; margin-bottom:10px;">
              <input v-model="manualUrl" placeholder="rtsp://admin:pass@192.168.0.104:554/Streaming/Channels/101" style="flex:1;" />
              <button :disabled="manualVerifying || !manualUrl" @click="verifyManualUrl" style="white-space:nowrap;">{{ manualVerifying ? '验证中…' : '验证' }}</button>
            </div>
            <div v-if="manualResult" style="margin-top:10px;">
              <div v-if="manualResult.success" style="display:flex; align-items:flex-start; gap:10px;">
                <div style="flex:1;">
                  <div style="color:var(--green); font-size:13px; margin-bottom:6px;">✅ 验证成功</div>
                  <div class="snapshot-box" v-if="manualResult.snapshot_base64" style="max-width:200px;">
                    <img :src="'data:image/jpeg;base64,' + manualResult.snapshot_base64" />
                  </div>
                </div>
                <button @click="saveManualTool" style="margin-top:4px;">保存为工具</button>
              </div>
              <div v-else style="color:#e74c3c; font-size:12px;">
                ❌ 验证失败: {{ manualResult.error || '未知错误' }}
              </div>
            </div>
          </div>
        </div>
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
            <span class="badge badge-blue">{{ t.tool_type === 'rtsp_camera' ? 'RTSP' : 'ONVIF' }}</span>
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
          <div class="info-row"><span class="info-label">通道:</span><span class="info-value">{{ t.channel || 1 }}</span></div>
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
.btn-wizard { background: #2e5c8b; color: #fff; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; overflow: auto; padding: 40px 0; }
.modal { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 24px; width: 640px; max-width: 90vw; }
.modal-wide { width: 900px; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 16px; }
.field { display: flex; flex-direction: column; gap: 4px; }
.field label { font-size: 12px; color: #888; }
.field input, .field select { background: #1e1e1e; border: 1px solid #3a3a3a; color: #e8e8e8; padding: 8px; border-radius: 6px; font-size: 13px; }
.field.full { grid-column: 1 / -1; }
.modal-actions { display: flex; gap: 10px; justify-content: flex-end; margin-top: 16px; }
.device-list { display: flex; flex-direction: column; gap: 8px; }
.device-card { background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 8px; padding: 12px; transition: border-color 0.15s; }
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
.info-label { color: #888; min-width: 80px; }
.info-value { color: #bbb; word-break: break-all; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-right: 6px; }
.badge-green { background: rgba(124,179,66,0.2); color: var(--green); }
.badge-gray { background: #333; color: #888; }
.badge-blue { background: rgba(66,133,244,0.2); color: #4285f4; }
.badge-yellow { background: rgba(255,193,7,0.15); color: #ffc107; }
.badge-red { background: rgba(244,67,54,0.15); color: #f44336; }
.empty { color: #666; padding: 40px; text-align: center; }
.hint-box { background: rgba(255,193,7,0.1); border: 1px solid rgba(255,193,7,0.3); border-radius: 8px; padding: 12px 16px; font-size: 13px; color: #ffc107; margin-bottom: 16px; line-height: 1.6; }

/* 向导样式 */
.wizard-step { margin-top: 16px; }
.probe-info { background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 8px; padding: 12px 16px; }
.probe-info .info-row { margin-bottom: 6px; }
.probe-info .info-label { min-width: 80px; }
.card-success { border-color: rgba(124,179,66,0.5); }
.card-failed { border-color: rgba(244,67,54,0.3); opacity: 0.8; }
.progress-bar { width: 100%; height: 4px; background: #333; border-radius: 2px; overflow: hidden; margin-top: 16px; }
.progress-fill { height: 100%; background: var(--green); animation: progress 1.5s ease-in-out infinite; width: 30%; }
@keyframes progress {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(400%); }
}

/* 通道网格 */
.channel-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; max-height: 520px; overflow-y: auto; padding-right: 4px; }
.channel-card { background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 8px; padding: 10px; }
.snapshot-box { text-align: center; background: #151515; border-radius: 6px; padding: 4px; }
.snapshot-box img { max-width: 100%; max-height: 120px; border-radius: 4px; }

@media (max-width: 900px) {
  .channel-grid { grid-template-columns: repeat(2, 1fr); }
}
@media (max-width: 600px) {
  .channel-grid { grid-template-columns: 1fr; }
}
</style>
