const cdpHost = "192.168.8.11:19223";

async function run() {
  const listRes = await fetch(`http://${cdpHost}/json/list`);
  const pages = await listRes.json();
  const page = pages.find(p => p.url && p.url.includes("txffp.com"));
  if (!page) {
    console.error("No txffp.com page found in browser!");
    return;
  }

  console.log("Found page:", page.id, page.title, page.url);
  
  // Fix port in webSocketDebuggerUrl
  let wsUrl = page.webSocketDebuggerUrl;
  if (!wsUrl.includes(":19223")) {
    wsUrl = wsUrl.replace("ws://192.168.8.11/", `ws://${cdpHost}/`);
  }
  console.log("Connecting to:", wsUrl);

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

  console.log("CDP WebSocket connected successfully.");

  // Evaluate document details (DOM only, no cookies, no storage)
  const evalResult = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const links = Array.from(document.querySelectorAll('a')).map(a => ({ text: a.innerText.trim(), href: a.href })).filter(l => l.text);
        const buttons = Array.from(document.querySelectorAll('button, input[type="button"], input[type="submit"]')).map(b => b.innerText || b.value || b.id || b.className);
        return {
          title: document.title,
          url: window.location.href,
          loginLinks: links.filter(l => l.text.includes("登录") || l.href.includes("login")),
          allNavLinks: links.slice(0, 15),
          buttons: buttons.slice(0, 10)
        };
      })()
    `,
    returnByValue: true
  });

  console.log("Page DOM inspection:", JSON.stringify(evalResult.result.value, null, 2));

  ws.close();
}

run().catch(console.error);
