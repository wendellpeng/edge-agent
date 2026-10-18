<script setup>
import { ref, onMounted } from "vue";
import { api } from "../api.js";

const scenarios = ref([]);
const tools = ref([]);
const showForm = ref(false);
const editing = ref(null);

const form = ref({
  id: 0, name: "", enabled: true,
  pipeline: [
    { node_id: "node_1", node_type: "trigger", config: { trigger_type: "cron", config: "0 */4 * * *" } },
    { node_id: "node_2", node_type: "tool", config: { tool_id: null } },
    { node_id: "node_3", node_type: "http_push", config: { url: "", method: "POST", timeout_secs: 30, retry_count: 3, headers: {}, image_param_name: "image", body_params: "" } },
  ],
});

async function load() {
  scenarios.value = await api.listScenarios();
  tools.value = await api.listTools();
}

function addNew() {
  form.value = {
    id: 0, name: "", enabled: true,
    pipeline: [
      { node_id: "node_1", node_type: "trigger", config: { trigger_type: "cron", config: "0 */4 * * *" } },
      { node_id: "node_2", node_type: "tool", config: { tool_id: null } },
      { node_id: "node_3", node_type: "http_push", config: { url: "", method: "POST", timeout_secs: 30, retry_count: 3, headers: {}, image_param_name: "image", body_params: "" } },
    ],
  };
  editing.value = null;
  showForm.value = true;
}

function edit(s) {
  form.value = JSON.parse(JSON.stringify(s));
  editing.value = s.id;
  showForm.value = true;
}

async function save() {
  try {
    const toolNode = form.value.pipeline.find(n => n.node_type === "tool");
    if (toolNode && !toolNode.config.tool_id) {
      alert("请选择一个工具");
      return;
    }
    const pushNode = form.value.pipeline.find(n => n.node_type === "http_push");
    if (pushNode && !pushNode.config.url) {
      alert("请填写推送 URL");
      return;
    }

    if (editing.value) await api.updateScenario(editing.value, form.value);
    else await api.addScenario(form.value);
    showForm.value = false;
    await load();
  } catch (e) { alert("保存失败: " + e); }
}

async function remove(id) { if (!confirm("确定删除？")) return; try { await api.deleteScenario(id); await load(); } catch (e) { alert(e); } }
async function runNow(id) { try { await api.runScenario(id); alert("已触发"); } catch (e) { alert(e); } }

function getNodeLabel(node) {
  if (node.node_type === "trigger") {
    const t = node.config.trigger_type || "cron";
    return `触发器 (${t === "cron" ? "定时" : t === "interval" ? "间隔" : "手动"})`;
  }
  if (node.node_type === "tool") {
    const toolId = node.config.tool_id;
    const tool = tools.value.find(t => t.id === toolId);
    return `抓拍: ${tool ? tool.name : "未选择"}`;
  }
  if (node.node_type === "http_push") {
    return `推送: ${node.config.url || "未配置"}`;
  }
  return node.node_type;
}

function getTriggerDesc(node) {
  if (node.node_type !== "trigger") return "";
  const t = node.config.trigger_type || "cron";
  const c = node.config.config || "";
  if (t === "cron") return `定时: ${c}`;
  if (t === "interval") return `间隔: ${c} 秒`;
  return "手动触发";
}

onMounted(load);
</script>

<template>
  <div>
    <header><h1>场景编排</h1><p class="sub">定时触发 → 工具执行 → HTTP 推送</p></header>
    <section class="toolbar">
      <button @click="addNew">+ 新建场景</button>
      <button class="btn-secondary" @click="load">刷新</button>
    </section>

    <!-- 场景编排表单 -->
    <div v-if="showForm" class="modal-overlay" @click.self="showForm = false">
      <div class="modal modal-wide">
        <h3>{{ editing ? "编辑场景" : "新建场景" }}</h3>

        <div class="form-section">
          <h4>基本信息</h4>
          <div class="form-grid">
            <div class="field"><label>场景名称</label><input v-model="form.name" /></div>
            <div class="field"><label>启用</label><input type="checkbox" v-model="form.enabled" /></div>
          </div>
        </div>

        <!-- 节点编排 -->
        <div class="form-section">
          <h4>节点编排</h4>
          <div class="pipeline">
            <div v-for="(node, idx) in form.pipeline" :key="node.node_id" class="pipeline-node">
              <div class="node-badge">{{ idx + 1 }}</div>

              <!-- 触发器节点 -->
              <div v-if="node.node_type === 'trigger'" class="node-content">
                <div class="node-title">触发器</div>
                <div class="form-grid small">
                  <div class="field">
                    <label>触发类型</label>
                    <select v-model="node.config.trigger_type">
                      <option value="cron">定时触发 (cron)</option>
                      <option value="interval">间隔触发 (秒)</option>
                      <option value="manual">手动触发</option>
                    </select>
                  </div>
                  <div class="field">
                    <label>配置</label>
                    <input v-model="node.config.config" :placeholder="node.config.trigger_type === 'cron' ? '0 */4 * * *' : '2'" />
                  </div>
                </div>
              </div>

              <!-- 工具节点 -->
              <div v-else-if="node.node_type === 'tool'" class="node-content">
                <div class="node-title">抓拍</div>
                <div class="field">
                  <label>选择摄像头</label>
                  <select v-model="node.config.tool_id">
                    <option :value="null">请选择</option>
                    <option v-for="t in tools" :key="t.id" :value="t.id">{{ t.name }} ({{ t.location || t.endpoint }})</option>
                  </select>
                </div>
              </div>

              <!-- HTTP 推送节点 -->
              <div v-else-if="node.node_type === 'http_push'" class="node-content">
                <div class="node-title">HTTP 推送</div>
                <div class="form-grid small">
                  <div class="field full"><label>推送 URL</label><input v-model="node.config.url" placeholder="http://backend/api/receive" /></div>
                  <div class="field">
                    <label>方法</label>
                    <select v-model="node.config.method"><option>POST</option><option>PUT</option></select>
                  </div>
                  <div class="field"><label>超时(秒)</label><input type="number" v-model="node.config.timeout_secs" /></div>
                  <div class="field"><label>重试次数</label><input type="number" v-model="node.config.retry_count" /></div>
                  <div class="field"><label>图片参数名</label><input v-model="node.config.image_param_name" placeholder="image" /></div>
                  <div class="field full">
                    <label>请求参数 (JSON)</label>
                    <textarea v-model="node.config.body_params" rows="3" placeholder='{"scene":"warehouse","device_id":"cam01"}'></textarea>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-actions">
          <button @click="save">保存</button>
          <button class="btn-secondary" @click="showForm = false">取消</button>
        </div>
      </div>
    </div>

    <!-- 场景列表 -->
    <div class="scenario-list">
      <div v-for="s in scenarios" :key="s.id" class="scenario-card">
        <div class="scenario-header">
          <div>
            <span class="scenario-name">{{ s.name }}</span>
            <span :class="['badge', s.enabled ? 'badge-green' : 'badge-gray']">{{ s.enabled ? "启用" : "禁用" }}</span>
          </div>
          <div class="scenario-actions">
            <button class="btn-small" @click="runNow(s.id)">立即执行</button>
            <button class="btn-small" @click="edit(s)">编辑</button>
            <button class="btn-small btn-danger" @click="remove(s.id)">删除</button>
          </div>
        </div>
        <div class="pipeline-preview">
          <div v-for="(node, idx) in s.pipeline" :key="node.node_id" class="preview-node">
            <div class="preview-step">{{ idx + 1 }}</div>
            <div class="preview-info">
              <div class="preview-title">{{ getNodeLabel(node) }}</div>
              <div v-if="node.node_type === 'trigger'" class="preview-desc">{{ getTriggerDesc(node) }}</div>
            </div>
            <div v-if="idx < s.pipeline.length - 1" class="preview-arrow">→</div>
          </div>
        </div>
      </div>
    </div>
    <p v-if="!scenarios.length" class="empty">暂无场景</p>
  </div>
</template>

<style scoped>
header h1 { font-size: 20px; margin: 0 0 4px; }
.sub { color: #888; font-size: 13px; margin: 0 0 16px; }
.toolbar { display: flex; gap: 10px; margin-bottom: 16px; }
button { background: var(--green); color: #10240a; border: none; padding: 8px 16px; border-radius: 6px; font-weight: 600; cursor: pointer; }
.btn-secondary { background: #3a3a3a; color: #e8e8e8; }
.btn-small { padding: 4px 10px; font-size: 12px; margin-right: 6px; }
.btn-danger { background: #8b2e2e; color: #fff; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; overflow: auto; padding: 40px 0; }
.modal { background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 24px; width: 640px; max-width: 90vw; }
.modal-wide { width: 760px; }
.form-section { margin-bottom: 20px; }
.form-section h4 { font-size: 13px; color: var(--green); margin: 0 0 10px; border-bottom: 1px solid var(--border); padding-bottom: 6px; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.form-grid.small { gap: 10px; }
.field { display: flex; flex-direction: column; gap: 4px; }
.field label { font-size: 12px; color: #888; }
.field input, .field select, .field textarea { background: #1e1e1e; border: 1px solid #3a3a3a; color: #e8e8e8; padding: 8px; border-radius: 6px; font-size: 13px; }
.field.full { grid-column: 1 / -1; }
.modal-actions { display: flex; gap: 10px; justify-content: flex-end; margin-top: 16px; }

.pipeline { display: flex; flex-direction: column; gap: 12px; }
.pipeline-node { display: flex; gap: 12px; align-items: flex-start; background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 10px; padding: 16px; }
.node-badge { width: 28px; height: 28px; background: var(--green); color: #10240a; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: 700; font-size: 13px; flex-shrink: 0; }
.node-content { flex: 1; }
.node-title { font-weight: 600; font-size: 14px; margin-bottom: 10px; color: var(--green); }

.scenario-list { display: flex; flex-direction: column; gap: 12px; }
.scenario-card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 10px; padding: 16px; }
.scenario-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.scenario-name { font-weight: 600; font-size: 15px; margin-right: 10px; }

.pipeline-preview { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.preview-node { display: flex; align-items: center; gap: 8px; background: #1e1e1e; border: 1px solid #3a3a3a; border-radius: 8px; padding: 8px 12px; }
.preview-step { width: 22px; height: 22px; background: #3a3a3a; color: #bbb; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 11px; flex-shrink: 0; }
.preview-title { font-size: 13px; font-weight: 500; }
.preview-desc { font-size: 11px; color: #888; }
.preview-arrow { color: var(--green); font-weight: 700; font-size: 16px; }

.badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-right: 6px; }
.badge-green { background: rgba(124,179,66,0.2); color: var(--green); }
.badge-gray { background: #333; color: #888; }
.empty { color: #666; padding: 40px; text-align: center; }
</style>
