const fs = require("fs");
const path = require("path");
const http = require("http");

const host = process.env.MEMFLOW_WEB_HOST || "127.0.0.1";
const port = Number(process.env.MEMFLOW_WEB_PORT || 5173);
const agentProxyOrigin = process.env.MEMFLOW_AGENT_PROXY || "http://127.0.0.1:3000";
const distDir = path.resolve(process.cwd(), "dist");

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
};

function sendJson(res, statusCode, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(body);
}

function serveFile(res, filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const contentType = contentTypes[ext] || "application/octet-stream";
  const stream = fs.createReadStream(filePath);

  stream.on("error", (error) => {
    sendJson(res, 500, { error: error.message });
  });

  res.writeHead(200, { "Content-Type": contentType });
  stream.pipe(res);
}

function resolveFilePath(urlPath) {
  const normalized = urlPath === "/" ? "/index.html" : urlPath;
  const safePath = path.normalize(normalized).replace(/^([.][.][/\\])+/, "");
  const candidate = path.join(distDir, safePath);

  if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
    return candidate;
  }

  return path.join(distDir, "index.html");
}

function proxyRequest(req, res) {
  const rewrittenPath = (req.url || "/").replace(/^\/api/, "") || "/";
  const upstream = new URL(rewrittenPath, agentProxyOrigin);
  const headers = { ...req.headers, host: upstream.host };
  const authHeader = headers.authorization;

  if (typeof authHeader === "string" && authHeader.startsWith("Bearer ") && !headers["x-api-key"]) {
    headers["x-api-key"] = authHeader.slice("Bearer ".length);
  }

  const options = {
    hostname: upstream.hostname,
    port: upstream.port || 80,
    path: upstream.pathname + upstream.search,
    method: req.method,
    headers,
  };

  const proxy = http.request(options, (upstreamRes) => {
    res.writeHead(upstreamRes.statusCode || 502, upstreamRes.headers);
    upstreamRes.pipe(res);
  });

  proxy.on("error", (error) => {
    sendJson(res, 502, { error: error.message });
  });

  req.pipe(proxy);
}

if (!fs.existsSync(distDir)) {
  console.error(`Missing dist directory: ${distDir}`);
  process.exit(1);
}

const server = http.createServer((req, res) => {
  const urlPath = (req.url || "/").split("?")[0];

  if (urlPath.startsWith("/api")) {
    proxyRequest(req, res);
    return;
  }

  const filePath = resolveFilePath(urlPath);
  serveFile(res, filePath);
});

server.listen(port, host, () => {
  console.log(`MemFlow web UI ready at http://${host}:${port}`);
  console.log(`Serving static files from ${distDir}`);
  console.log(`Proxying /api/* requests to ${agentProxyOrigin}`);
});
