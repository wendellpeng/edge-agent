const API_BASE = "";

async function request(method, path, body) {
  const opts = {
    method,
    headers: { "Content-Type": "application/json" },
  };
  if (body) opts.body = JSON.stringify(body);

  const resp = await fetch(`${API_BASE}${path}`, opts);
  const data = await resp.json().catch(() => ({}));

  if (!resp.ok || !data.success) {
    throw new Error(data.error || `HTTP ${resp.status}`);
  }
  return data.data;
}

export const api = {
  // 工具发现
  discoverTools: (timeout) => request("GET", `/api/tools/discover?timeout=${timeout}`),

  // 工具管理
  listTools: () => request("GET", "/api/tools"),
  addTool: (tool) => request("POST", "/api/tools", tool),
  updateTool: (id, tool) => request("PUT", `/api/tools/${id}`, tool),
  deleteTool: (id) => request("DELETE", `/api/tools/${id}`),
  testSnapshot: (id) => request("POST", `/api/tools/${id}/snapshot`),
  listChannels: (id) => request("GET", `/api/tools/${id}/channels`),
  listRtspTemplates: () => request("GET", "/api/tools/rtsp-templates"),
  generateRtspUrl: (params) => request("POST", "/api/tools/rtsp-url", params),
  probeDevice: (params) => request("POST", "/api/tools/probe", params),
  verifyCandidates: (params) => request("POST", "/api/tools/verify", params),

  // 场景
  listScenarios: () => request("GET", "/api/scenarios"),
  addScenario: (s) => request("POST", "/api/scenarios", s),
  updateScenario: (id, s) => request("PUT", `/api/scenarios/${id}`, s),
  deleteScenario: (id) => request("DELETE", `/api/scenarios/${id}`),
  runScenario: (id) => request("POST", `/api/scenarios/${id}/run`),

  // 任务
  listTasks: () => request("GET", "/api/tasks"),
};
