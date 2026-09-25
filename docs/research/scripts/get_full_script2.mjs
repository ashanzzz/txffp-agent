const cdpHost = "192.168.8.11:19223";

async function run() {
  const listRes = await fetch(`http://${cdpHost}/json/list`);
  const pages = await listRes.json();
  const page = pages.find(p => p.url && p.url.includes("txffp.com"));

  let wsUrl = page.webSocketDebuggerUrl;
  if (!wsUrl.includes(":19223")) {
    wsUrl = wsUrl.replace("ws://192.168.8.11/", `ws://${cdpHost}/`);
  }

  const ws = new WebSocket(wsUrl);
  let messageId = 0;
  const pending = new Map();

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++messageId;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.id && pending.has(data.id)) {
      const { resolve, reject } = pending.get(data.id);
      pending.delete(data.id);
      if (data.error) reject(data.error);
      else resolve(data.result);
    }
  };

  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });

  const res = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const inlineScripts = Array.from(document.querySelectorAll('script:not([src])')).map(s => s.innerText);
        return inlineScripts[1] || "";
      })()
    `,
    returnByValue: true
  });

  console.log("Full login script:\n", res.result.value);
  ws.close();
}

run().catch(console.error);
