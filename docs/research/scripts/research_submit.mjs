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

  const evalResult = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const submitElements = Array.from(document.querySelectorAll('a, button, input[type="submit"], input[type="button"], div'))
          .filter(el => {
            const text = (el.innerText || el.value || "").trim();
            return text === "授权登录" || text === "登录" || el.id.includes("login") || el.id.includes("btn") || el.className.includes("btn") || el.className.includes("login_b");
          })
          .map(el => ({
            tag: el.tagName,
            id: el.id,
            className: el.className,
            text: (el.innerText || el.value || "").trim(),
            onclick: el.getAttribute('onclick'),
            href: el.getAttribute('href')
          }));

        return submitElements;
      })()
    `,
    returnByValue: true
  });

  console.log("Login submit elements:", JSON.stringify(evalResult.result.value, null, 2));
  ws.close();
}

run().catch(console.error);
